//! HTTP layer (M2 slice). Routes follow `docs/M1_API_CONTRACT.md`; deviations are listed in `docs/M2_SLICE_NOTES.md`.
//! Rules enforced here: workspace comes ONLY from the session cookie; foreign ids look like missing ids (404);
//! mutating requests need `X-Requested-With: zyn`; no server path or core (Turkish) text is ever sent to clients.

use crate::store::{BackupMeta, FileMeta, StoreError, WorkspaceStore};
use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Read};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio_util::io::ReaderStream;
use zyn_doc_engine::{Document, DocumentSummary, EngineError, EntryView, ImportItem, SettingsView};

const COOKIE_NAME: &str = "zyn_session";
pub const MAX_OPEN_DOCUMENTS: usize = 8;

struct OpenDoc {
    file_id: String,
    doc: Arc<Mutex<Document>>,
}

#[derive(Default)]
struct SessionDocs {
    next_id: u32,
    docs: HashMap<u32, OpenDoc>,
}

pub struct AppState {
    pub store: WorkspaceStore,
    docs: Mutex<HashMap<String, SessionDocs>>,
}

#[derive(Clone)]
struct SessionToken(String);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AppState {
    pub fn new(store: WorkspaceStore) -> Arc<Self> {
        Arc::new(Self { store, docs: Mutex::new(HashMap::new()) })
    }

    /// Expires old workspaces and drops their open documents.
    pub fn sweep(&self) -> usize {
        let tokens = self.store.sweep();
        let mut docs = lock(&self.docs);
        for t in &tokens {
            docs.remove(t);
        }
        tokens.len()
    }

    fn delete_session(&self, token: &str) {
        self.store.delete_session(token);
        lock(&self.docs).remove(token);
    }

    fn get_doc(&self, token: &str, id: u32) -> Result<Arc<Mutex<Document>>, ApiError> {
        lock(&self.docs).get(token).and_then(|s| s.docs.get(&id)).map(|d| d.doc.clone()).ok_or_else(ApiError::not_found)
    }
}

// ---------- errors ----------

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into() }
    }
    fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", "The requested item was not found.")
    }
    fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal error occurred.")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": { "code": self.code, "message": self.message } }))).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        let status = match e {
            StoreError::NotFound => StatusCode::NOT_FOUND,
            StoreError::UploadTooLarge | StoreError::QuotaExceeded => StatusCode::PAYLOAD_TOO_LARGE,
            StoreError::TooManyFiles => StatusCode::CONFLICT,
            StoreError::TooManyWorkspaces => StatusCode::SERVICE_UNAVAILABLE,
            StoreError::InvalidName => StatusCode::BAD_REQUEST,
            StoreError::Io => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self::new(status, e.code(), e.message())
    }
}

impl From<EngineError> for ApiError {
    fn from(e: EngineError) -> Self {
        if let Some(core) = &e.core_message {
            eprintln!("engine error {}: {core}", e.code); // server log only
        }
        let status = match e.code {
            "RPF_INVALID" | "RPF_CORRUPT" | "RPF_ENCRYPTED" => StatusCode::UNPROCESSABLE_ENTITY,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "INVALID_PATH" | "INVALID_REQUEST" | "INVALID_NAME" => StatusCode::BAD_REQUEST,
            "CONFLICT" | "NOTHING_TO_UNDO" | "NOTHING_TO_REDO" => StatusCode::CONFLICT,
            "STALE_DOCUMENT" => StatusCode::PRECONDITION_FAILED,
            "EXPORT_TOO_LARGE" => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self::new(status, e.code, e.message)
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, ApiError> + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f).await.map_err(|_| ApiError::internal())?
}

// ---------- session + hardening middleware ----------

fn cookie_value(req: &Request) -> Option<String> {
    let raw = req.headers().get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').filter_map(|p| p.trim().split_once('=')).find(|(k, _)| *k == COOKIE_NAME).map(|(_, v)| v.to_string())
}

async fn session_layer(State(st): State<Arc<AppState>>, mut req: Request, next: Next) -> Response {
    if !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS)
        && req.headers().get("x-requested-with").and_then(|v| v.to_str().ok()) != Some("zyn")
    {
        return ApiError::new(StatusCode::FORBIDDEN, "CSRF_REJECTED", "Missing X-Requested-With header.").into_response();
    }
    let presented = cookie_value(&req);
    let st2 = st.clone();
    let p2 = presented.clone();
    let token = match tokio::task::spawn_blocking(move || st2.store.session(p2.as_deref())).await {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => return ApiError::from(e).into_response(),
        Err(_) => return ApiError::internal().into_response(),
    };
    req.extensions_mut().insert(SessionToken(token.clone()));
    let mut resp = next.run(req).await;
    if presented.as_deref() != Some(token.as_str()) {
        let secure = if st.store.cfg.cookie_secure { "; Secure" } else { "" };
        let c = format!("{COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}{secure}", st.store.cfg.max_age.as_secs());
        if let Ok(v) = HeaderValue::from_str(&c) {
            resp.headers_mut().append(header::SET_COOKIE, v);
        }
    }
    let h = resp.headers_mut();
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    resp
}

// ---------- DTOs ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileDto {
    file_id: String,
    name: String,
    size: u64,
}

impl From<FileMeta> for FileDto {
    fn from(m: FileMeta) -> Self {
        Self { file_id: m.file_id, name: m.name, size: m.size }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRequest {
    file_id: String,
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
}

#[derive(Deserialize)]
struct VersionQuery {
    version: Option<String>,
}

#[derive(Deserialize)]
struct DirQuery {
    dir: Option<String>,
}

#[derive(Deserialize)]
struct PathsBody {
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct TextBody {
    path: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameBody {
    path: String,
    new_name: String,
}

#[derive(Deserialize)]
struct SettingsBody {
    path: String,
    values: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct MkdirBody {
    path: String,
}

#[derive(Deserialize)]
struct MoveBody {
    paths: Vec<String>,
    dir: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreBody {
    backup_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResult {
    summary: DocumentSummary,
    old_size: u64,
    new_size: u64,
    backup_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupDto {
    backup_id: String,
    size: u64,
    created_unix: u64,
}

impl From<BackupMeta> for BackupDto {
    fn from(b: BackupMeta) -> Self {
        Self { backup_id: b.id, size: b.size, created_unix: b.created_unix }
    }
}

/// Optional optimistic-concurrency check: `If-Match: <revision>` must equal the document's current revision.
fn if_match(h: &HeaderMap) -> Result<Option<u64>, ApiError> {
    match h.get(header::IF_MATCH) {
        None => Ok(None),
        Some(v) => v
            .to_str()
            .ok()
            .and_then(|s| s.trim().trim_matches('"').parse::<u64>().ok())
            .map(Some)
            .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "If-Match must be a document revision number.")),
    }
}

/// Runs a mutating engine operation on a document in the blocking pool and returns the fresh summary.
async fn mutate(
    st: &Arc<AppState>,
    token: &str,
    id: u32,
    headers: &HeaderMap,
    op: impl FnOnce(&mut Document) -> Result<(), EngineError> + Send + 'static,
) -> Result<Json<DocumentSummary>, ApiError> {
    let doc = st.get_doc(token, id)?;
    let expected = if_match(headers)?;
    Ok(Json(
        blocking(move || {
            let mut d = lock(&doc);
            d.ensure_revision(expected)?;
            op(&mut d)?;
            Ok(d.summary(id))
        })
        .await?,
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenListItem {
    doc_id: u32,
    name: String,
    file_id: String,
}

// ---------- helpers ----------

fn pct_encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn attachment(name: &str) -> HeaderValue {
    let ascii: String = name.chars().map(|c| if (c.is_ascii_graphic() || c == ' ') && c != '"' && c != '\\' && c != '%' { c } else { '_' }).collect();
    HeaderValue::from_str(&format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{}", pct_encode(name)))
        .unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

fn bytes_download(name: &str, bytes: Vec<u8>) -> Response {
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    r.headers_mut().insert(header::CONTENT_DISPOSITION, attachment(name));
    r
}

// ---------- handlers ----------

const DEV_HTML: &str = include_str!("../static/dev.html");
const DEV_JS: &str = include_str!("../static/dev.js");

/// Strict CSP: no inline script, no third-party origins. The harness is same-origin only.
fn static_response(content_type: &'static str, body: &'static str) -> Response {
    let mut r = Response::new(Body::from(body));
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    r
}

async fn dev_page() -> Response {
    static_response("text/html; charset=utf-8", DEV_HTML)
}

async fn dev_script() -> Response {
    static_response("text/javascript; charset=utf-8", DEV_JS)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(st): State<Arc<AppState>>) -> Response {
    let st2 = st.clone();
    match tokio::task::spawn_blocking(move || st2.store.ready()).await {
        Ok(true) => (StatusCode::OK, "ready").into_response(),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "storage not writable").into_response(),
    }
}

async fn get_session(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let c = &st.store.cfg;
    Json(json!({ "limits": {
        "maxUploadBytes": c.max_upload_bytes, "maxWorkspaceBytes": c.max_workspace_bytes,
        "maxFiles": c.max_files_per_workspace, "maxOpenDocuments": MAX_OPEN_DOCUMENTS,
        "idleTtlSeconds": c.idle_ttl.as_secs(), "maxAgeSeconds": c.max_age.as_secs() } }))
}

async fn delete_session(State(st): State<Arc<AppState>>, Extension(SessionToken(t)): Extension<SessionToken>) -> StatusCode {
    let _ = tokio::task::spawn_blocking(move || st.delete_session(&t)).await;
    StatusCode::NO_CONTENT
}

/// Bridges async multipart chunks to the blocking, size-capped `store.add_file` reader.
struct ChanReader {
    rx: tokio::sync::mpsc::Receiver<Result<Vec<u8>, ()>>,
    buf: Vec<u8>,
    pos: usize,
}

impl Read for ChanReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.pos < self.buf.len() {
                let n = out.len().min(self.buf.len() - self.pos);
                out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
                self.pos += n;
                return Ok(n);
            }
            match self.rx.blocking_recv() {
                Some(Ok(chunk)) => {
                    self.buf = chunk;
                    self.pos = 0;
                }
                // An interrupted upload must fail, never commit a truncated file as if it were complete.
                Some(Err(())) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "upload interrupted")),
                None => return Ok(0),
            }
        }
    }
}

async fn upload(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(token)): Extension<SessionToken>,
    mut mp: Multipart,
) -> Result<Json<Vec<FileDto>>, ApiError> {
    let bad = || ApiError::new(StatusCode::BAD_REQUEST, "BAD_UPLOAD", "The upload request is malformed.");
    let mut out = Vec::new();
    while let Some(mut field) = mp.next_field().await.map_err(|_| bad())? {
        let Some(name) = field.file_name().map(str::to_owned) else { continue };
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, ()>>(4);
        let (st2, t2) = (st.clone(), token.clone());
        let handle = tokio::task::spawn_blocking(move || {
            let mut reader = ChanReader { rx, buf: Vec::new(), pos: 0 };
            st2.store.add_file(&t2, &name, &mut reader)
        });
        let mut interrupted = false;
        loop {
            match field.chunk().await {
                Ok(Some(c)) => {
                    if tx.send(Ok(c.to_vec())).await.is_err() {
                        break; // store stopped reading (limit hit / error); its result is reported below
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    interrupted = true;
                    let _ = tx.send(Err(())).await;
                    break;
                }
            }
        }
        drop(tx);
        let res = handle.await.map_err(|_| ApiError::internal())?;
        if interrupted {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, "UPLOAD_INTERRUPTED", "The upload was interrupted."));
        }
        out.push(FileDto::from(res?));
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(Json(out))
}

async fn list_files(State(st): State<Arc<AppState>>, Extension(SessionToken(t)): Extension<SessionToken>) -> Result<Json<Vec<FileDto>>, ApiError> {
    Ok(Json(st.store.list_files(&t)?.into_iter().map(FileDto::from).collect()))
}

async fn download_file(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(t)): Extension<SessionToken>,
    Path(file_id): Path<String>,
    Query(q): Query<VersionQuery>,
) -> Result<Response, ApiError> {
    let path = match q.version.as_deref() {
        None | Some("original") => st.store.original_path(&t, &file_id)?,
        Some("current") => st.store.current_path(&t, &file_id)?,
        Some(_) => return Err(ApiError::new(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Unknown version.")),
    };
    let name = st.store.list_files(&t)?.into_iter().find(|f| f.file_id == file_id).map(|f| f.name).ok_or_else(ApiError::not_found)?;
    let file = tokio::fs::File::open(&path).await.map_err(|_| ApiError::internal())?;
    let len = file.metadata().await.map(|m| m.len()).map_err(|_| ApiError::internal())?;
    let mut r = Response::new(Body::from_stream(ReaderStream::new(file)));
    r.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    r.headers_mut().insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    r.headers_mut().insert(header::CONTENT_DISPOSITION, attachment(&name));
    Ok(r)
}

async fn open_document(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(t)): Extension<SessionToken>,
    Json(req): Json<OpenRequest>,
) -> Result<Json<DocumentSummary>, ApiError> {
    // Idempotent per fileId: one open document per file.
    let existing = lock(&st.docs).get(&t).and_then(|s| s.docs.iter().find(|(_, d)| d.file_id == req.file_id).map(|(id, d)| (*id, d.doc.clone())));
    if let Some((id, doc)) = existing {
        return Ok(Json(blocking(move || Ok(lock(&doc).summary(id))).await?));
    }
    let path = st.store.current_path(&t, &req.file_id)?;
    let name = st.store.list_files(&t)?.into_iter().find(|f| f.file_id == req.file_id).map(|f| f.name).ok_or_else(ApiError::not_found)?;
    if lock(&st.docs).get(&t).map_or(0, |s| s.docs.len()) >= MAX_OPEN_DOCUMENTS {
        return Err(ApiError::new(StatusCode::CONFLICT, "TOO_MANY_DOCUMENTS", "Too many documents are open. Close one first."));
    }
    let doc = blocking(move || {
        let bytes: Arc<[u8]> = std::fs::read(&path).map_err(|_| ApiError::from(StoreError::Io))?.into();
        Ok(Document::open(&name, bytes)?)
    })
    .await?;
    let doc = Arc::new(Mutex::new(doc));
    // Register under the lock; the guard is released before any await. If a concurrent open of the same file won
    // the race, reuse the winner's document instead of keeping two.
    let (id, doc) = register_document(&st, &t, &req.file_id, doc);
    Ok(Json(blocking(move || Ok(lock(&doc).summary(id))).await?))
}

fn register_document(st: &AppState, token: &str, file_id: &str, doc: Arc<Mutex<Document>>) -> (u32, Arc<Mutex<Document>>) {
    let mut all = lock(&st.docs);
    let s = all.entry(token.to_string()).or_default();
    if let Some((id, d)) = s.docs.iter().find(|(_, d)| d.file_id == file_id).map(|(i, d)| (*i, d.doc.clone())) {
        return (id, d);
    }
    s.next_id += 1;
    let id = s.next_id;
    s.docs.insert(id, OpenDoc { file_id: file_id.to_string(), doc: doc.clone() });
    (id, doc)
}

async fn list_documents(State(st): State<Arc<AppState>>, Extension(SessionToken(t)): Extension<SessionToken>) -> Json<Vec<OpenListItem>> {
    let all = lock(&st.docs);
    let mut v: Vec<OpenListItem> = all
        .get(&t)
        .map(|s| s.docs.iter().map(|(id, d)| OpenListItem { doc_id: *id, name: lock(&d.doc).name.clone(), file_id: d.file_id.clone() }).collect())
        .unwrap_or_default();
    v.sort_by_key(|i| i.doc_id);
    Json(v)
}

async fn get_document(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(t)): Extension<SessionToken>,
    Path(id): Path<u32>,
) -> Result<Json<DocumentSummary>, ApiError> {
    let doc = st.get_doc(&t, id)?;
    Ok(Json(blocking(move || Ok(lock(&doc).summary(id))).await?))
}

async fn close_document(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(t)): Extension<SessionToken>,
    Path(id): Path<u32>,
) -> Result<StatusCode, ApiError> {
    lock(&st.docs).get_mut(&t).and_then(|s| s.docs.remove(&id)).map(|_| StatusCode::NO_CONTENT).ok_or_else(ApiError::not_found)
}

async fn download_entry(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(t)): Extension<SessionToken>,
    Path(id): Path<u32>,
    Query(q): Query<PathQuery>,
) -> Result<Response, ApiError> {
    let doc = st.get_doc(&t, id)?;
    let base = q.path.rsplit('/').next().unwrap_or("entry").to_string();
    let bytes = blocking(move || Ok(lock(&doc).read_file(&q.path)?)).await?;
    Ok(bytes_download(&base, bytes))
}

/// M2 addition (not in the M1 contract): re-serialized archive, verified by re-parse. Does not modify the stored upload.
async fn export_document(
    State(st): State<Arc<AppState>>,
    Extension(SessionToken(t)): Extension<SessionToken>,
    Path(id): Path<u32>,
) -> Result<Response, ApiError> {
    let doc = st.get_doc(&t, id)?;
    let (name, bytes) = blocking(move || {
        let d = lock(&doc);
        Ok((d.name.clone(), d.serialize_verified()?))
    })
    .await?;
    Ok(bytes_download(&name, bytes))
}


type Tok = Extension<SessionToken>;
type St = State<Arc<AppState>>;

async fn view_entry(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, Query(q): Query<PathQuery>) -> Result<Json<EntryView>, ApiError> {
    let doc = st.get_doc(&t, id)?;
    Ok(Json(blocking(move || Ok(lock(&doc).entry_view(&q.path)?)).await?))
}

async fn get_settings(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, Query(q): Query<PathQuery>) -> Result<Json<SettingsView>, ApiError> {
    let doc = st.get_doc(&t, id)?;
    Ok(Json(blocking(move || Ok(lock(&doc).settings_view(&q.path)?)).await?))
}

async fn put_settings(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<SettingsBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.settings_apply(&b.path, b.values)).await
}

async fn put_text(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<TextBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.write_text(&b.path, b.text)).await
}

async fn create_text(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<TextBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.create_text(&b.path, b.text)).await
}

async fn delete_entries(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<PathsBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.delete(b.paths)).await
}

async fn rename_entry(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<RenameBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.rename(&b.path, &b.new_name)).await
}

async fn mkdir_entry(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<MkdirBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.mkdir(&b.path)).await
}

async fn move_entries(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap, Json(b): Json<MoveBody>) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, move |d| d.move_to(b.paths, &b.dir)).await
}

async fn undo(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, |d| if d.undo() { Ok(()) } else { Err(EngineError::new("NOTHING_TO_UNDO", "There is nothing to undo.")) }).await
}

async fn redo(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap) -> Result<Json<DocumentSummary>, ApiError> {
    mutate(&st, &t, id, &h, |d| if d.redo() { Ok(()) } else { Err(EngineError::new("NOTHING_TO_REDO", "There is nothing to redo.")) }).await
}

/// Reads the multipart body into memory (bounded by the per-request upload cap): `(file name, bytes)` per file part.
async fn read_parts(st: &AppState, mut mp: Multipart) -> Result<Vec<(String, Vec<u8>)>, ApiError> {
    let bad = || ApiError::new(StatusCode::BAD_REQUEST, "BAD_UPLOAD", "The upload request is malformed.");
    let cap = st.store.cfg.max_upload_bytes;
    let mut total: u64 = 0;
    let mut out = Vec::new();
    while let Some(mut field) = mp.next_field().await.map_err(|_| bad())? {
        let Some(name) = field.file_name().map(str::to_owned) else { continue };
        let mut data = Vec::new();
        while let Some(c) = field.chunk().await.map_err(|_| bad())? {
            total += c.len() as u64;
            if total > cap {
                return Err(StoreError::UploadTooLarge.into());
            }
            data.extend_from_slice(&c);
        }
        out.push((name, data));
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(out)
}

/// Import: each part's file name is its relative path inside the target folder (`?dir=`; default archive root).
async fn import_entries(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, Query(q): Query<DirQuery>, h: HeaderMap, mp: Multipart) -> Result<Json<DocumentSummary>, ApiError> {
    let parts = read_parts(&st, mp).await?;
    let dir = q.dir.unwrap_or_default();
    let items: Vec<ImportItem> = parts.into_iter().map(|(rel_path, data)| ImportItem { rel_path, data }).collect();
    mutate(&st, &t, id, &h, move |d| d.import(&dir, items).map(|_| ())).await
}

/// Replace one entry (`?path=`) with the single uploaded part.
async fn replace_entry(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, Query(q): Query<PathQuery>, h: HeaderMap, mp: Multipart) -> Result<Json<DocumentSummary>, ApiError> {
    let mut parts = read_parts(&st, mp).await?;
    if parts.len() != 1 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Send exactly one file."));
    }
    let (_, data) = parts.remove(0);
    mutate(&st, &t, id, &h, move |d| d.replace_file(&q.path, data)).await
}

async fn export_entries(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, Json(b): Json<PathsBody>) -> Result<Response, ApiError> {
    let doc = st.get_doc(&t, id)?;
    let max = st.store.cfg.max_upload_bytes;
    let (name, bytes, count) = blocking(move || {
        let d = lock(&doc);
        let stem = d.name.rsplit_once('.').map_or(d.name.clone(), |(s, _)| s.to_string());
        let (bytes, count) = d.export_zip(b.paths, max)?;
        Ok((format!("{stem}.zip"), bytes, count))
    })
    .await?;
    let mut r = bytes_download(&name, bytes);
    r.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    r.headers_mut().insert("x-zyn-file-count", HeaderValue::from(count as u64));
    Ok(r)
}

fn file_id_of(st: &AppState, t: &str, id: u32) -> Result<String, ApiError> {
    lock(&st.docs).get(t).and_then(|s| s.docs.get(&id)).map(|d| d.file_id.clone()).ok_or_else(ApiError::not_found)
}

/// serialize -> re-parse verify -> (backup previous version) -> atomic write -> mark clean. The original upload is never modified.
async fn save_document(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, h: HeaderMap) -> Result<Json<SaveResult>, ApiError> {
    let doc = st.get_doc(&t, id)?;
    let file_id = file_id_of(&st, &t, id)?;
    let expected = if_match(&h)?;
    let st2 = st.clone();
    Ok(Json(
        blocking(move || {
            let mut d = lock(&doc);
            d.ensure_revision(expected)?;
            let bytes = d.serialize_verified()?;
            let (old_size, new_size, backup_id) = st2.store.save_version(&t, &file_id, &bytes)?;
            d.mark_saved(Arc::from(bytes))?;
            Ok(SaveResult { summary: d.summary(id), old_size, new_size, backup_id })
        })
        .await?,
    ))
}

async fn list_backups(State(st): St, Extension(SessionToken(t)): Tok, Path(file_id): Path<String>) -> Result<Json<Vec<BackupDto>>, ApiError> {
    Ok(Json(st.store.list_backups(&t, &file_id)?.into_iter().map(BackupDto::from).collect()))
}

async fn download_backup(State(st): St, Extension(SessionToken(t)): Tok, Path((file_id, backup_id)): Path<(String, String)>) -> Result<Response, ApiError> {
    let path = st.store.backup_path(&t, &file_id, &backup_id)?;
    let name = st.store.file_meta(&t, &file_id)?.name;
    let bytes = blocking(move || std::fs::read(path).map_err(|_| ApiError::internal())).await?;
    Ok(bytes_download(&name, bytes))
}

/// Restore: the backup becomes the live version (the version it replaces is itself backed up) and the document is reloaded
/// from it; undo history is cleared because the restored state is a new baseline.
async fn restore_backup(State(st): St, Extension(SessionToken(t)): Tok, Path(id): Path<u32>, Json(b): Json<RestoreBody>) -> Result<Json<DocumentSummary>, ApiError> {
    let doc = st.get_doc(&t, id)?;
    let file_id = file_id_of(&st, &t, id)?;
    let st2 = st.clone();
    Ok(Json(
        blocking(move || {
            let path = st2.store.backup_path(&t, &file_id, &b.backup_id)?;
            let bytes = std::fs::read(path).map_err(|_| ApiError::internal())?;
            let mut d = lock(&doc);
            let arc: Arc<[u8]> = Arc::from(bytes);
            // Validate before touching anything on disk.
            Document::open("verify", arc.clone())?;
            st2.store.save_version(&t, &file_id, &arc)?;
            d.reload(arc)?;
            Ok(d.summary(id))
        })
        .await?,
    ))
}

async fn api_not_found() -> ApiError {
    ApiError::not_found()
}

pub fn router(st: Arc<AppState>) -> Router {
    let upload_limit = st.store.cfg.max_workspace_bytes.min(usize::MAX as u64) as usize;
    let api = Router::new()
        .route("/session", get(get_session).delete(delete_session))
        .route("/files", get(list_files).post(upload).layer(DefaultBodyLimit::max(upload_limit)))
        .route("/files/{file_id}/download", get(download_file))
        .route("/documents", get(list_documents).post(open_document))
        .route("/documents/{id}", get(get_document).delete(close_document))
        .route("/documents/{id}/entry", get(download_entry))
        .route("/documents/{id}/entries", get(view_entry))
        .route("/documents/{id}/entries/raw", get(download_entry))
        .route("/documents/{id}/entries/text", axum::routing::put(put_text).post(create_text))
        .route("/documents/{id}/settings", get(get_settings).put(put_settings))
        .route("/documents/{id}/import", post(import_entries).layer(DefaultBodyLimit::max(upload_limit)))
        .route("/documents/{id}/replace", post(replace_entry).layer(DefaultBodyLimit::max(upload_limit)))
        .route("/documents/{id}/export", get(export_document).post(export_entries))
        .route("/documents/{id}/delete", post(delete_entries))
        .route("/documents/{id}/rename", post(rename_entry))
        .route("/documents/{id}/mkdir", post(mkdir_entry))
        .route("/documents/{id}/move", post(move_entries))
        .route("/documents/{id}/undo", post(undo))
        .route("/documents/{id}/redo", post(redo))
        .route("/documents/{id}/save", post(save_document))
        .route("/documents/{id}/restore", post(restore_backup))
        .route("/files/{file_id}/backups", get(list_backups))
        .route("/files/{file_id}/backups/{backup_id}/download", get(download_backup))
        .fallback(api_not_found) // unknown /api/v1 paths are JSON 404s, never the SPA page
        .layer(middleware::from_fn_with_state(st.clone(), session_layer));
    let base = Router::new().route("/healthz", get(healthz)).route("/readyz", get(readyz)).nest("/api/v1", api);
    let base = match st.store.cfg.static_dir.clone() {
        Some(dir) => {
            use tower_http::services::{ServeDir, ServeFile};
            // Real files are served as-is; any other non-API path gets index.html so client-side routes survive a reload.
            let spa = ServeDir::new(&dir).fallback(ServeFile::new(dir.join("index.html")));
            base.route("/dev", get(dev_page)).route("/dev.js", get(dev_script)).fallback_service(spa)
        }
        None => base.route("/", get(dev_page)).route("/dev.js", get(dev_script)),
    };
    base.with_state(st)
}
