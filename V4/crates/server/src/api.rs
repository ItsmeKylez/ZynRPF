//! API v1 (M1 subset): anonymous sessions, workspaces, resumable uploads, parse jobs, browsing, Range reads.
use crate::http::*;
use crate::json::*;
use crate::sha256::Sha256;
use rpf_core::{Index, Limits};
use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod edit;

const MAX_RANGE: u64 = 1 << 20;
const MAX_JSON: u64 = 16 * 1024;
const CACHE_CAP: usize = 8;

pub struct Cfg {
    pub data_dir: PathBuf,
    pub max_upload: u64,
    pub quota: u64,
    /// Total bytes across all sessions; 0 disables the cap. Protects the volume from filling up.
    pub global_quota: u64,
    pub chunk: u64,
    pub idle: Duration,
    pub upload_ttl: Duration,
    pub secure: bool,
    pub limits: Limits,
    pub max_workspaces: usize,
    pub max_sessions: usize,
    pub rate_reads: u32,
    pub rate_writes: u32,
    pub max_blob: u64,
}

struct Session {
    id: String,
    csrf: String,
    last: Instant,
    win_start: Instant,
    reads: u32,
    writes: u32,
}
struct Ws {
    id: String,
    owner: String,
    name: String,
    created: u64,
    versions: Vec<String>,
}
#[derive(Clone)]
enum VState {
    Parsing,
    Ready,
    Failed(String, String, String),
}
struct Ver {
    id: String,
    ws: String,
    owner: String,
    path: PathBuf,
    size: u64,
    filename: String,
    state: VState,
    entries: u64,
    origin: &'static str,
    created: u64,
}
struct Up {
    ws: String,
    owner: String,
    filename: String,
    size: u64,
    recv: BTreeSet<u64>,
    path: PathBuf,
    sha: Option<String>,
    created: Instant,
    completed: bool,
}
struct Job {
    owner: String,
    kind: &'static str,
    state: &'static str,
    progress: u32,
    error: Option<(String, String, String)>,
    version: Option<String>,
    report: Option<String>,
}
#[derive(Default)]
struct Inner {
    sessions: HashMap<String, Session>, // key: sha256(token)
    ws: HashMap<String, Ws>,
    vers: HashMap<String, Ver>,
    ups: HashMap<String, Up>,
    jobs: HashMap<String, Job>,
    cache: Vec<(String, Arc<Index>)>,
    created_window: Option<(Instant, u32)>,
    edits: HashMap<String, edit::Edit>,
    blobs: HashMap<u64, edit::Blob>,
    writers: HashMap<String, (String, Instant)>,
}

pub struct State {
    pub cfg: Cfg,
    inner: Mutex<Inner>,
    pub conns: AtomicUsize,
    counter: AtomicU64,
    /// Static web UI files loaded once at startup from WEB_DIR (path -> bytes, content type).
    pub web: HashMap<String, (std::sync::Arc<Vec<u8>>, &'static str)>,
}

pub fn rand_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).expect("random source unavailable (/dev/urandom)");
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
fn sha_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.hex()
}
fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
fn bad(code: &'static str, msg: &str, next: &str) -> ApiErr {
    err(400, code, msg, next)
}
fn ws_json(w: &Ws, used: u64) -> String {
    format!("{{\"id\":{},\"name\":{},\"created_at\":{},\"version_ids\":[{}],\"usage_bytes\":{}}}", q(&w.id), q(&w.name), w.created,
        w.versions.iter().map(|v| q(v)).collect::<Vec<_>>().join(","), used)
}
fn job_json(id: &str, j: &Job) -> String {
    let e = match &j.error {
        Some((c, m, n)) => format!("{{\"code\":{},\"message\":{},\"next_step\":{}}}", q(c), q(m), q(n)),
        None => "null".into(),
    };
    let v = j.version.as_ref().map(|v| q(v)).unwrap_or("null".into());
    let rep = j.report.as_ref().map(|r| format!(",\"report\":{}", r)).unwrap_or_default();
    format!("{{\"id\":{},\"type\":{},\"state\":{},\"progress\":{},\"error\":{},\"version_id\":{}{}}}", q(id), q(j.kind), q(j.state), j.progress, e, v, rep)
}
fn entry_json(i: &rpf_core::EntryInfo, path: Option<&str>, detected: Option<&str>) -> String {
    let opt = |v: Option<u32>| v.map(|x| x.to_string()).unwrap_or("null".into());
    let mut s = format!(
        "{{\"id\":\"{}\",\"name\":{},\"kind\":{},\"stored_size\":{},\"uncompressed_size\":{},\"compressed\":{},\"encryption_tag\":{},\"offset\":{},\"child_count\":{},\"sys_flags\":{},\"gfx_flags\":{},\"large_resource\":{}",
        i.id, q(&i.name), q(i.kind), i.stored_size, i.uncompressed_size, i.compressed, opt(i.encryption_tag), i.offset, opt(i.child_count), opt(i.sys_flags), opt(i.gfx_flags), i.large_resource
    );
    if let Some(p) = path {
        s += &format!(",\"path\":{}", q(p));
    }
    if let Some(d) = detected {
        s += &format!(",\"detected_type\":{}", q(d));
    }
    s + "}"
}

impl State {
    pub fn new(cfg: Cfg, web: HashMap<String, (std::sync::Arc<Vec<u8>>, &'static str)>) -> State {
        State { cfg, inner: Mutex::new(Inner::default()), conns: AtomicUsize::new(0), counter: AtomicU64::new(0), web }
    }
    pub fn request_id(&self) -> String {
        format!("{}-{:06}", &rand_hex(4), self.counter.fetch_add(1, Ordering::Relaxed))
    }
    fn cookie_name(&self) -> &'static str {
        if self.cfg.secure { "__Host-sid" } else { "sid" }
    }

    // ---------- auth ----------
    /// Returns the session id. Enforces cookie, CSRF on state-changing requests, and rate limits.
    fn auth(&self, req: &Req, bulk_upload: bool) -> Result<String, ApiErr> {
        let tok = req.cookie(self.cookie_name()).ok_or_else(|| err(401, "unauthenticated", "You need to start a session first.", "Start a new session and try again."))?;
        let key = sha_hex(&tok);
        let mut g = self.inner.lock().unwrap();
        let s = g.sessions.get_mut(&key).ok_or_else(|| err(401, "unauthenticated", "Your session has expired.", "Start a new session. Files from the old session are deleted."))?;
        if s.last.elapsed() > self.cfg.idle {
            return Err(err(401, "unauthenticated", "Your session has expired.", "Start a new session. Files from the old session are deleted."));
        }
        let mutating = !matches!(req.method.as_str(), "GET" | "HEAD");
        if mutating {
            let ok = req.headers.get("x-csrf-token").map_or(false, |t| *t == s.csrf);
            if !ok {
                return Err(err(403, "csrf_invalid", "The request was missing a valid security token.", "Reload the page and try again."));
            }
        }
        s.last = Instant::now();
        if s.win_start.elapsed() > Duration::from_secs(60) {
            s.win_start = Instant::now();
            s.reads = 0;
            s.writes = 0;
        }
        if !bulk_upload {
            let over = if mutating { s.writes += 1; s.writes > self.cfg.rate_writes } else { s.reads += 1; s.reads > self.cfg.rate_reads };
            if over {
                let mut e = err(429, "rate_limited", "You are sending requests too quickly.", "Wait a moment and try again.");
                e.retry_after = Some(60u64.saturating_sub(s.win_start.elapsed().as_secs()).max(1));
                return Err(e);
            }
        }
        Ok(s.id.clone())
    }

    fn usage_all(g: &Inner) -> u64 {
        let v: u64 = g.vers.values().filter(|v| !matches!(v.state, VState::Failed(..))).map(|v| v.size).sum();
        let u: u64 = g.ups.values().filter(|u| !u.completed).map(|u| u.size).sum();
        let b: u64 = g.blobs.values().map(|b| b.size).sum();
        v + u + b
    }
    fn server_full(&self, g: &Inner, extra: u64) -> Result<(), ApiErr> {
        if self.cfg.global_quota > 0 && Self::usage_all(g) + extra > self.cfg.global_quota {
            return Err(err(503, "server_full", "The service has no free storage right now.", "Try again later. Files are deleted automatically after a period of inactivity."));
        }
        Ok(())
    }

    fn usage(g: &Inner, sid: &str) -> u64 {
        let v: u64 = g.vers.values().filter(|v| v.owner == sid && !matches!(v.state, VState::Failed(..))).map(|v| v.size).sum();
        let u: u64 = g.ups.values().filter(|u| u.owner == sid && !u.completed).map(|u| u.size).sum();
        let b: u64 = g.blobs.values().filter(|b| b.owner == sid).map(|b| b.size).sum();
        v + u + b
    }

    // ---------- janitor / deletion ----------
    fn remove_ws_locked(&self, g: &mut Inner, wsid: &str) {
        if let Some(w) = g.ws.remove(wsid) {
            for v in &w.versions {
                g.vers.remove(v);
                g.cache.retain(|(id, _)| id != v);
            }
        }
        g.ups.retain(|_, u| u.ws != wsid);
        g.edits.remove(wsid);
        g.writers.remove(wsid);
        g.blobs.retain(|_, b| b.ws != wsid);
        let _ = fs::remove_dir_all(self.cfg.data_dir.join(wsid));
    }
    pub fn janitor(&self) {
        let mut g = self.inner.lock().unwrap();
        let expired: Vec<String> = g.sessions.iter().filter(|(_, s)| s.last.elapsed() > self.cfg.idle).map(|(k, _)| k.clone()).collect();
        for k in expired {
            if let Some(s) = g.sessions.remove(&k) {
                let wss: Vec<String> = g.ws.values().filter(|w| w.owner == s.id).map(|w| w.id.clone()).collect();
                for w in wss {
                    self.remove_ws_locked(&mut g, &w);
                }
                g.jobs.retain(|_, j| j.owner != s.id);
            }
        }
        let old: Vec<String> = g.ups.iter().filter(|(_, u)| !u.completed && u.created.elapsed() > self.cfg.upload_ttl).map(|(k, _)| k.clone()).collect();
        for k in old {
            if let Some(u) = g.ups.remove(&k) {
                let _ = fs::remove_file(u.path);
            }
        }
    }

    // ---------- index access ----------
    fn index_for(&self, sid: &str, vid: &str) -> Result<(Arc<Index>, PathBuf), ApiErr> {
        let (path, state) = {
            let g = self.inner.lock().unwrap();
            let v = g.vers.get(vid).filter(|v| v.owner == sid).ok_or_else(not_found)?;
            if let Some((_, i)) = g.cache.iter().find(|(id, _)| id == vid) {
                return Ok((i.clone(), v.path.clone()));
            }
            (v.path.clone(), v.state.clone())
        };
        match state {
            VState::Ready => {}
            VState::Parsing => return Err(err(409, "version_not_ready", "This archive is still being checked.", "Wait for the job to finish, then try again.")),
            VState::Failed(c, m, n) => {
                let mut e = err(409, "version_failed", m, &n);
                e.details = Some(format!("{{\"check_code\":{}}}", q(&c)));
                return Err(e);
            }
        }
        let mut f = File::open(&path).map_err(|_| not_found())?;
        let idx = Arc::new(Index::parse(&mut std::io::BufReader::new(&mut f), &self.cfg.limits).map_err(|c| err(409, "version_failed", c.message(), c.next_step()))?);
        let mut g = self.inner.lock().unwrap();
        g.cache.retain(|(id, _)| id != vid);
        g.cache.push((vid.to_string(), idx.clone()));
        while g.cache.len() > CACHE_CAP {
            g.cache.remove(0);
        }
        Ok((idx, path))
    }

    // ---------- parse job ----------
    fn run_parse(self: &Arc<Self>, job: String, vid: String) {
        let (path, sha, owner) = {
            let mut g = self.inner.lock().unwrap();
            if let Some(j) = g.jobs.get_mut(&job) {
                j.state = "running";
                j.progress = 10;
            }
            let v = g.vers.get(&vid).unwrap();
            (v.path.clone(), None::<String>, v.owner.clone())
        };
        let _ = (sha, owner);
        let res: Result<(Arc<Index>, u64), (String, String, String)> = (|| {
            let mut f = File::open(&path).map_err(|_| ("IO_ERROR".to_string(), rpf_core::Code::Io.message().to_string(), rpf_core::Code::Io.next_step().to_string()))?;
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            let idx = Index::parse(&mut std::io::BufReader::new(&mut f), &self.cfg.limits).map_err(|c| (c.as_str().to_string(), c.message().to_string(), c.next_step().to_string()))?;
            Ok((Arc::new(idx), len))
        })();
        let mut g = self.inner.lock().unwrap();
        match res {
            Ok((idx, _)) => {
                if let Some(v) = g.vers.get_mut(&vid) {
                    v.state = VState::Ready;
                    v.entries = idx.len() as u64;
                }
                g.cache.push((vid.clone(), idx));
                while g.cache.len() > CACHE_CAP {
                    g.cache.remove(0);
                }
                if let Some(j) = g.jobs.get_mut(&job) {
                    j.state = "succeeded";
                    j.progress = 100;
                    j.version = Some(vid);
                }
            }
            Err((c, m, n)) => {
                if let Some(v) = g.vers.get_mut(&vid) {
                    v.state = VState::Failed(c.clone(), m.clone(), n.clone());
                    let _ = fs::remove_file(&v.path); // failed uploads do not keep consuming storage
                }
                if let Some(j) = g.jobs.get_mut(&job) {
                    j.state = "failed";
                    j.progress = 100;
                    j.error = Some((c, m, n));
                    j.version = Some(vid);
                }
            }
        }
    }

    // ---------- routing ----------
    pub fn route(self: &Arc<Self>, req: &mut Req) -> Result<Resp, ApiErr> {
        let path = req.path.clone();
        if req.method == "GET" && !path.starts_with("/api/") {
            // Only files that were loaded from WEB_DIR at startup can be served, so no request path ever reaches the file system.
            let key = if path == "/" { "/index.html" } else { path.as_str() };
            return match self.web.get(key) {
                Some((data, ctype)) => {
                    let mut r = Resp { status: 200, headers: vec![], body: Body::Bytes { data: data.clone(), ctype } };
                    r.headers.push(("Cache-Control".into(), if key.starts_with("/assets/") { "public, max-age=31536000, immutable".into() } else { "no-cache".into() }));
                    Ok(r)
                }
                None if path == "/" => Ok(Resp::json(200, "{\"service\":\"rpf-editor\",\"api\":\"/api/v1\",\"ui\":\"not built; set WEB_DIR to the web/dist folder\"}".into())),
                None => Err(not_found()),
            };
        }
        let Some(rest) = path.strip_prefix("/api/v1/") else { return Err(not_found()) };
        let seg: Vec<&str> = rest.trim_end_matches('/').split('/').collect();
        let m = req.method.clone();
        match (m.as_str(), seg.as_slice()) {
            ("GET", ["health", "live"]) => Ok(Resp::json(200, "{\"status\":\"ok\"}".into())),
            ("GET", ["health", "ready"]) => {
                let p = self.cfg.data_dir.join(".ready");
                if fs::write(&p, b"ok").is_ok() { Ok(Resp::json(200, "{\"status\":\"ready\"}".into())) } else { Err(err(503, "not_ready", "Storage is not writable.", "")) }
            }
            ("GET", ["version"]) => Ok(Resp::json(200, format!("{{\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION")))),
            ("GET", ["limits"]) => {
                let l = &self.cfg.limits;
                Ok(Resp::json(200, format!(
                    "{{\"max_upload_bytes\":{},\"chunk_size_bytes\":{},\"max_entries\":{},\"max_names_bytes\":{},\"max_depth\":{},\"session_quota_bytes\":{},\"max_range_bytes\":{},\"idle_hours\":{},\"max_workspaces\":{},\"max_blob_bytes\":{},\"max_ops\":{},\"max_versions\":{}}}",
                    self.cfg.max_upload, self.cfg.chunk, l.max_entries, l.max_names_bytes, l.max_depth, self.cfg.quota, MAX_RANGE, self.cfg.idle.as_secs() / 3600, self.cfg.max_workspaces, self.cfg.max_blob, edit::MAX_OPS, edit::MAX_VERSIONS)))
            }
            ("POST", ["sessions", "anonymous"]) => self.new_session(req),
            _ => self.route_authed(req, &seg),
        }
    }

    fn new_session(&self, req: &mut Req) -> Result<Resp, ApiErr> {
        let _ = req.body(MAX_JSON);
        let tok = rand_hex(32);
        let csrf = rand_hex(16);
        let sid = rand_hex(16);
        {
            let mut g = self.inner.lock().unwrap();
            let now = Instant::now();
            let (ws, n) = g.created_window.unwrap_or((now, 0));
            let (ws, n) = if ws.elapsed() > Duration::from_secs(60) { (now, 0) } else { (ws, n) };
            if n >= 60 || g.sessions.len() >= self.cfg.max_sessions {
                let mut e = err(429, "rate_limited", "Too many sessions are being created right now.", "Wait a minute and try again.");
                e.retry_after = Some(60);
                return Err(e);
            }
            g.created_window = Some((ws, n + 1));
            g.sessions.insert(sha_hex(&tok), Session { id: sid, csrf: csrf.clone(), last: now, win_start: now, reads: 0, writes: 0 });
        }
        let secure = if self.cfg.secure { "; Secure" } else { "" };
        let mut r = Resp::json(201, format!("{{\"csrf_token\":{},\"idle_expiry_seconds\":{}}}", q(&csrf), self.cfg.idle.as_secs()));
        r.headers.push(("Set-Cookie".into(), format!("{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400{}", self.cookie_name(), tok, secure)));
        Ok(r)
    }

    fn route_authed(self: &Arc<Self>, req: &mut Req, seg: &[&str]) -> Result<Resp, ApiErr> {
        let is_chunk = matches!(seg, ["uploads", _, "chunks", _]) && req.method == "PUT";
        let sid = self.auth(req, is_chunk)?;
        let m = req.method.clone();
        match (m.as_str(), seg) {
            ("GET", ["me"]) => {
                let g = self.inner.lock().unwrap();
                let n = g.ws.values().filter(|w| w.owner == sid).count();
                // The CSRF token is returned here so a reloaded page can keep its existing session (the caller already holds the session cookie).
                let csrf = g.sessions.values().find(|s| s.id == sid).map(|s| s.csrf.clone()).unwrap_or_default();
                Ok(Resp::json(200, format!("{{\"anonymous\":true,\"usage_bytes\":{},\"quota_bytes\":{},\"workspace_count\":{},\"csrf_token\":{}}}", Self::usage(&g, &sid), self.cfg.quota, n, q(&csrf))))
            }
            ("DELETE", ["me"]) | ("DELETE", ["sessions", "current"]) => {
                let mut g = self.inner.lock().unwrap();
                let wss: Vec<String> = g.ws.values().filter(|w| w.owner == sid).map(|w| w.id.clone()).collect();
                for w in wss {
                    self.remove_ws_locked(&mut g, &w);
                }
                g.sessions.retain(|_, s| s.id != sid);
                g.jobs.retain(|_, j| j.owner != sid);
                Ok(Resp::empty(204))
            }
            ("POST", ["workspaces"]) => {
                let body = req.body(MAX_JSON)?;
                let o = parse_flat(&String::from_utf8_lossy(&body)).ok_or_else(|| bad("invalid_json", "The request body is not valid.", "Send a JSON object."))?;
                let name = get_str(&o, "name").unwrap_or_default();
                if name.trim().is_empty() || name.chars().count() > 100 || name.chars().any(|c| c.is_control()) {
                    return Err(bad("invalid_name", "The workspace name must be 1 to 100 characters with no control characters.", "Choose a different name."));
                }
                let mut g = self.inner.lock().unwrap();
                if g.ws.values().filter(|w| w.owner == sid).count() >= self.cfg.max_workspaces {
                    return Err(err(409, "limit_exceeded", format!("You can have {} workspace(s) in an anonymous session.", self.cfg.max_workspaces), "Delete the existing workspace first."));
                }
                let id = rand_hex(16);
                fs::create_dir_all(self.cfg.data_dir.join(&id)).map_err(|_| err(503, "storage_error", "Storage is unavailable.", "Try again later."))?;
                let w = Ws { id: id.clone(), owner: sid.clone(), name, created: now_unix(), versions: vec![] };
                let j = ws_json(&w, 0);
                g.ws.insert(id, w);
                Ok(Resp::json(201, j))
            }
            ("GET", ["workspaces"]) => {
                let g = self.inner.lock().unwrap();
                let l: Vec<String> = g.ws.values().filter(|w| w.owner == sid).map(|w| ws_json(w, 0)).collect();
                Ok(Resp::json(200, format!("{{\"workspaces\":[{}]}}", l.join(","))))
            }
            ("GET", ["workspaces", id]) => {
                let g = self.inner.lock().unwrap();
                let w = g.ws.get(*id).filter(|w| w.owner == sid).ok_or_else(not_found)?;
                let used: u64 = w.versions.iter().filter_map(|v| g.vers.get(v)).filter(|v| !matches!(v.state, VState::Failed(..))).map(|v| v.size).sum();
                Ok(Resp::json(200, ws_json(w, used)))
            }
            ("DELETE", ["workspaces", id]) => {
                let mut g = self.inner.lock().unwrap();
                g.ws.get(*id).filter(|w| w.owner == sid).ok_or_else(not_found)?;
                self.remove_ws_locked(&mut g, id);
                Ok(Resp::empty(204))
            }
            ("GET", ["workspaces", id, "versions"]) => {
                let g = self.inner.lock().unwrap();
                let w = g.ws.get(*id).filter(|w| w.owner == sid).ok_or_else(not_found)?;
                let l: Vec<String> = w.versions.iter().filter_map(|v| g.vers.get(v)).map(|v| {
                    let st = match &v.state { VState::Parsing => "parsing", VState::Ready => "ready", VState::Failed(..) => "failed" };
                    format!("{{\"id\":{},\"filename\":{},\"size\":{},\"state\":{},\"entry_count\":{},\"origin\":{},\"created_at\":{}}}", q(&v.id), q(&v.filename), v.size, q(st), v.entries, q(v.origin), v.created)
                }).collect();
                Ok(Resp::json(200, format!("{{\"versions\":[{}]}}", l.join(","))))
            }
            ("POST", ["workspaces", id, "uploads"]) => self.upload_start(req, &sid, id),
            ("PUT", ["uploads", uid, "chunks", idx]) => self.upload_chunk(req, &sid, uid, idx),
            ("GET", ["uploads", uid]) => {
                let g = self.inner.lock().unwrap();
                let u = g.ups.get(*uid).filter(|u| u.owner == sid).ok_or_else(not_found)?;
                let r: Vec<String> = u.recv.iter().map(|x| x.to_string()).collect();
                Ok(Resp::json(200, format!("{{\"size\":{},\"chunk_size\":{},\"received\":[{}],\"completed\":{}}}", u.size, self.cfg.chunk, r.join(","), u.completed)))
            }
            ("DELETE", ["uploads", uid]) => {
                let mut g = self.inner.lock().unwrap();
                g.ups.get(*uid).filter(|u| u.owner == sid && !u.completed).ok_or_else(not_found)?;
                if let Some(u) = g.ups.remove(*uid) {
                    let _ = fs::remove_file(u.path);
                }
                Ok(Resp::empty(204))
            }
            ("POST", ["uploads", uid, "complete"]) => self.upload_complete(req, &sid, uid),
            ("GET", ["jobs", id]) => {
                let g = self.inner.lock().unwrap();
                let j = g.jobs.get(*id).filter(|j| j.owner == sid).ok_or_else(not_found)?;
                Ok(Resp::json(200, job_json(id, j)))
            }
            ("GET", ["versions", vid, "entries"]) => self.list_entries(req, &sid, vid),
            ("GET", ["versions", vid, "entries", eid]) => self.entry_detail(&sid, vid, eid),
            ("GET", ["versions", vid, "entries", eid, "path"]) => {
                let (idx, _) = self.index_for(&sid, vid)?;
                let id = parse_id(&idx, eid)?;
                let mut chain = vec![];
                let mut cur = id;
                loop {
                    chain.push(cur);
                    match idx.parent(cur) { Some(p) => cur = p, None => break }
                    if chain.len() > 1024 { break; }
                }
                chain.reverse();
                let l: Vec<String> = chain.iter().map(|c| format!("{{\"id\":\"{}\",\"name\":{}}}", c, q(idx.name(*c)))).collect();
                Ok(Resp::json(200, format!("{{\"path\":{},\"chain\":[{}]}}", q(&idx.path(id)), l.join(","))))
            }
            ("GET", ["versions", vid, "search"]) => {
                let (idx, _) = self.index_for(&sid, vid)?;
                let qs = req.query.get("q").cloned().unwrap_or_default();
                if qs.is_empty() || qs.chars().count() > 100 {
                    return Err(bad("invalid_query", "The search text must be 1 to 100 characters.", "Enter a shorter search."));
                }
                let limit = req.query.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(400).clamp(1, 400);
                let hits = idx.search(&qs, limit);
                let l: Vec<String> = hits.iter().filter_map(|h| idx.info(*h).map(|i| entry_json(&i, Some(&idx.path(*h)), None))).collect();
                Ok(Resp::json(200, format!("{{\"hits\":[{}],\"truncated\":{}}}", l.join(","), hits.len() >= limit)))
            }
            ("GET", ["versions", vid, "entries", eid, "bytes"]) => self.entry_bytes(req, &sid, vid, eid),
            ("POST", ["versions", vid, "entries", eid, "open"]) => self.open_nested(req, &sid, vid, eid),
            ("GET", ["versions", vid, "download"]) => {
                let (idx, path) = self.index_for(&sid, vid)?;
                let len = idx.file_len();
                let (start, n, status, cr) = range_of(req, len, false)?;
                let fname = {
                    let g = self.inner.lock().unwrap();
                    g.vers.get(*vid).map(|v| safe_filename(&v.filename)).unwrap_or("archive.rpf".into())
                };
                let mut r = Resp { status, headers: vec![("Content-Disposition".into(), format!("attachment; filename=\"{}\"", fname))], body: Body::File { path, start, len: n } };
                if let Some(c) = cr { r.headers.push(("Content-Range".into(), c)); }
                r.headers.push(("Accept-Ranges".into(), "bytes".into()));
                Ok(r)
            }
            _ => self.route_edit(req, &sid, seg),
        }
    }

    fn upload_start(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let body = req.body(MAX_JSON)?;
        let o = parse_flat(&String::from_utf8_lossy(&body)).ok_or_else(|| bad("invalid_json", "The request body is not valid.", "Send a JSON object."))?;
        let filename = get_str(&o, "filename").unwrap_or_default();
        let size = get_u64(&o, "size").unwrap_or(0);
        let sha = get_str(&o, "sha256").map(|s| s.to_lowercase());
        if filename.is_empty() || filename.chars().count() > 255 || filename.chars().any(|c| c.is_control()) {
            return Err(bad("invalid_filename", "The file name must be 1 to 255 characters with no control characters.", "Rename the file and try again."));
        }
        if size == 0 {
            return Err(bad("invalid_size", "The file size must be greater than zero.", "Choose a different file."));
        }
        if let Some(s) = &sha {
            if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(bad("invalid_hash", "The SHA-256 value must be 64 hexadecimal characters.", "Fix the value or leave it out."));
            }
        }
        if size > self.cfg.max_upload {
            let mut e = err(413, "limit_exceeded", format!("The file is {} bytes. The upload limit is {} bytes.", size, self.cfg.max_upload), "Upload a smaller archive.");
            e.details = Some(format!("{{\"limit\":\"max_upload_bytes\",\"value\":{},\"max\":{}}}", size, self.cfg.max_upload));
            return Err(e);
        }
        let mut g = self.inner.lock().unwrap();
        g.ws.get(wsid).filter(|w| w.owner == sid).ok_or_else(not_found)?;
        let used = Self::usage(&g, sid);
        self.server_full(&g, size)?;
        if used + size > self.cfg.quota {
            let mut e = err(413, "quota_exceeded", format!("This upload would use {} bytes of your {} byte storage allowance ({} already used).", size, self.cfg.quota, used), "Delete the workspace or a version to free space.");
            e.details = Some(format!("{{\"limit\":\"session_quota_bytes\",\"value\":{},\"max\":{}}}", used + size, self.cfg.quota));
            return Err(e);
        }
        let uid = rand_hex(16);
        let path = self.cfg.data_dir.join(wsid).join(format!("upload-{}.part", uid));
        let f = File::create(&path).map_err(|_| err(503, "storage_error", "Storage is unavailable.", "Try again later."))?;
        f.set_len(size).map_err(|_| err(503, "storage_error", "Storage is unavailable.", "Try again later."))?;
        g.ups.insert(uid.clone(), Up { ws: wsid.into(), owner: sid.into(), filename, size, recv: BTreeSet::new(), path, sha, created: Instant::now(), completed: false });
        Ok(Resp::json(201, format!("{{\"upload_id\":{},\"chunk_size\":{},\"chunk_count\":{}}}", q(&uid), self.cfg.chunk, (size + self.cfg.chunk - 1) / self.cfg.chunk)))
    }

    fn upload_chunk(&self, req: &mut Req, sid: &str, uid: &str, idx: &str) -> Result<Resp, ApiErr> {
        let idx: u64 = idx.parse().map_err(|_| bad("invalid_index", "The chunk number is not valid.", ""))?;
        let (size, path) = {
            let g = self.inner.lock().unwrap();
            let u = g.ups.get(uid).filter(|u| u.owner == sid && !u.completed).ok_or_else(not_found)?;
            (u.size, u.path.clone())
        };
        let start = idx.checked_mul(self.cfg.chunk).filter(|s| *s < size).ok_or_else(|| bad("invalid_index", "The chunk number is outside this upload.", "Use the chunk count from the start response."))?;
        let expect = self.cfg.chunk.min(size - start);
        if req.content_length != expect {
            let mut e = bad("length_mismatch", &format!("This chunk must be exactly {} bytes.", expect), "Split the file using the chunk size from the start response.");
            e.details = Some(format!("{{\"expected\":{},\"received\":{}}}", expect, req.content_length));
            return Err(e);
        }
        let mut f = OpenOptions::new().write(true).open(&path).map_err(|_| err(503, "storage_error", "Storage is unavailable.", "Try again later."))?;
        f.seek(SeekFrom::Start(start)).map_err(|_| err(503, "storage_error", "Storage is unavailable.", "Try again later."))?;
        // a chunk counts as received only after every byte arrived; interrupted chunks are simply re-sent
        req.stream_body_to(&mut f).map_err(|_| bad("body_incomplete", "The chunk did not arrive completely.", "Send this chunk again."))?;
        let mut g = self.inner.lock().unwrap();
        if let Some(u) = g.ups.get_mut(uid) {
            u.recv.insert(idx);
        }
        Ok(Resp::empty(204))
    }

    fn upload_complete(self: &Arc<Self>, req: &mut Req, sid: &str, uid: &str) -> Result<Resp, ApiErr> {
        let _ = req.body(MAX_JSON);
        let mut g = self.inner.lock().unwrap();
        let (wsid, size, filename, part, sha) = {
            let u = g.ups.get(uid).filter(|u| u.owner == sid && !u.completed).ok_or_else(not_found)?;
            let need = (u.size + self.cfg.chunk - 1) / self.cfg.chunk;
            if (u.recv.len() as u64) != need {
                let mut e = err(409, "upload_incomplete", format!("{} of {} chunks have arrived.", u.recv.len(), need), "Send the missing chunks (see the upload status), then complete again.");
                e.details = Some(format!("{{\"received\":{},\"expected\":{}}}", u.recv.len(), need));
                return Err(e);
            }
            (u.ws.clone(), u.size, u.filename.clone(), u.path.clone(), u.sha.clone())
        };
        let vid = rand_hex(16);
        let jid = rand_hex(16);
        let vpath = self.cfg.data_dir.join(&wsid).join(format!("v-{}.rpf", vid));
        fs::rename(&part, &vpath).map_err(|_| err(503, "storage_error", "Storage is unavailable.", "Try again later."))?;
        if let Some(u) = g.ups.get_mut(uid) {
            u.completed = true;
        }
        g.vers.insert(vid.clone(), Ver { id: vid.clone(), ws: wsid.clone(), owner: sid.into(), path: vpath.clone(), size, filename, state: VState::Parsing, entries: 0, origin: "upload", created: now_unix() });
        if let Some(w) = g.ws.get_mut(&wsid) {
            w.versions.push(vid.clone());
        }
        g.jobs.insert(jid.clone(), Job { owner: sid.into(), kind: "parse", state: "queued", progress: 0, error: None, version: Some(vid.clone()), report: None });
        drop(g);
        let me = self.clone();
        let (j2, v2) = (jid.clone(), vid.clone());
        std::thread::spawn(move || {
            if let Some(expect) = sha {
                // optional integrity check requested by the client
                let mut h = Sha256::new();
                if let Ok(mut f) = File::open(&vpath) {
                    let mut b = vec![0u8; 1 << 20];
                    loop {
                        match f.read(&mut b) { Ok(0) | Err(_) => break, Ok(k) => h.update(&b[..k]) }
                    }
                }
                if h.hex() != expect {
                    let mut g = me.inner.lock().unwrap();
                    let (c, m, n) = ("UPLOAD_HASH_MISMATCH".to_string(), "The uploaded file does not match the SHA-256 value you provided.".to_string(), "Upload the file again.".to_string());
                    if let Some(v) = g.vers.get_mut(&v2) {
                        v.state = VState::Failed(c.clone(), m.clone(), n.clone());
                        let _ = fs::remove_file(&v.path);
                    }
                    if let Some(j) = g.jobs.get_mut(&j2) {
                        j.state = "failed";
                        j.progress = 100;
                        j.error = Some((c, m, n));
                    }
                    return;
                }
            }
            me.run_parse(j2, v2);
        });
        Ok(Resp::json(202, format!("{{\"job_id\":{},\"version_id\":{}}}", q(&jid), q(&vid))))
    }

    fn list_entries(&self, req: &Req, sid: &str, vid: &str) -> Result<Resp, ApiErr> {
        let (idx, _) = self.index_for(sid, vid)?;
        let parent = match req.query.get("parent") { Some(p) => parse_id(&idx, p)?, None => 0 };
        let (first, count) = idx.children(parent).ok_or_else(|| err(422, "not_a_folder", "That entry is not a folder.", "Choose a folder."))?;
        let limit = req.query.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(100).clamp(1, 500);
        let cursor = req.query.get("cursor").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let sort = req.query.get("sort").map(|s| s.as_str()).unwrap_or("name");
        let desc = req.query.get("order").map(|s| s == "desc").unwrap_or(false);
        let mut ids: Vec<u32> = (first..first + count).collect();
        match sort {
            "name" => ids.sort_by_cached_key(|i| (!matches!(idx.entry(*i).map(|e| e.kind), Some(rpf_core::Kind::Dir { .. })), idx.name(*i).to_lowercase())),
            "size" => ids.sort_by_key(|i| idx.info(*i).map(|x| x.stored_size).unwrap_or(0)),
            "kind" => ids.sort_by_cached_key(|i| (idx.info(*i).map(|x| x.kind).unwrap_or(""), idx.name(*i).to_lowercase())),
            _ => return Err(bad("invalid_sort", "Sort by name, size or kind.", "")),
        }
        if desc {
            ids.reverse();
        }
        let total = ids.len();
        let page: Vec<String> = ids.iter().skip(cursor).take(limit).filter_map(|i| idx.info(*i)).map(|i| entry_json(&i, None, None)).collect();
        let next = if cursor + limit < total { (cursor + limit).to_string() } else { "null".into() };
        let next = if next == "null" { next } else { q(&next) };
        Ok(Resp::json(200, format!("{{\"parent\":\"{}\",\"total\":{},\"entries\":[{}],\"next_cursor\":{}}}", parent, total, page.join(","), next)))
    }

    fn entry_detail(&self, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let info = idx.info(id).ok_or_else(not_found)?;
        let mut detected = match info.kind {
            "directory" => "folder",
            "resource" => "game_resource",
            _ => "binary",
        };
        if info.kind == "file" && info.stored_size >= 4 && !info.compressed {
            if let Ok(mut f) = File::open(&path) {
                let mut b = [0u8; 512];
                if f.seek(SeekFrom::Start(info.offset)).is_ok() {
                    let n = f.read(&mut b).unwrap_or(0).min(info.stored_size as usize);
                    if n >= 4 && u32::from_le_bytes([b[0], b[1], b[2], b[3]]) == rpf_core::format::MAGIC {
                        detected = "nested_rpf";
                    } else if n > 0 && b[..n].iter().all(|c| matches!(*c, 9 | 10 | 13 | 0x20..=0x7e)) {
                        detected = "text";
                    }
                }
            }
        } else if info.kind == "file" && info.compressed {
            detected = "compressed_binary";
        }
        Ok(Resp::json(200, entry_json(&info, Some(&idx.path(id)), Some(detected))))
    }

    /// Copies a stored (uncompressed, unencrypted) nested RPF out of its parent into a new read-only version.
    /// The parent archive is never touched, and the copy counts against the session quota.
    fn open_nested(&self, req: &mut Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let _ = req.body(MAX_JSON);
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let info = idx.info(id).ok_or_else(not_found)?;
        let not_nested = || err(422, "not_a_nested_archive", "This entry is not a stored RPF archive.", "Choose an entry that is detected as a nested RPF archive.");
        if info.kind != "file" || info.large_resource {
            return Err(not_nested());
        }
        if info.compressed || info.encryption_tag != Some(0) {
            return Err(err(422, "nested_not_supported", "This nested archive is compressed or encrypted, so it cannot be opened yet.", "Download the entry bytes instead."));
        }
        let (off, len) = idx.stored_span(id).ok_or_else(not_found)?;
        if len < 16 || len > self.cfg.max_upload {
            return Err(not_nested());
        }
        let mut src = File::open(&path).map_err(|_| not_found())?;
        src.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
        let mut magic = [0u8; 4];
        src.read_exact(&mut magic).map_err(|_| not_nested())?;
        if u32::from_le_bytes(magic) != rpf_core::format::MAGIC {
            return Err(not_nested());
        }
        src.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
        let (wsid, filename) = {
            let g = self.inner.lock().unwrap();
            let v = g.vers.get(vid).filter(|v| v.owner == sid).ok_or_else(not_found)?;
            if Self::usage(&g, sid) + len > self.cfg.quota {
                return Err(err(413, "quota_exceeded", "Opening this archive would exceed your storage quota.", "Delete a version or workspace, then try again."));
            }
            self.server_full(&g, len)?;
            if g.ws.get(&v.ws).map_or(0, |w| w.versions.len()) >= edit::MAX_VERSIONS {
                return Err(err(409, "version_cap", format!("A workspace can keep at most {} versions.", edit::MAX_VERSIONS), "Delete an older version, then try again."));
            }
            (v.ws.clone(), info.name.clone())
        };
        let nid = rand_hex(16);
        let npath = self.cfg.data_dir.join(&wsid).join(format!("v-{}.rpf", nid));
        let copied = (|| -> std::io::Result<()> {
            let mut out = File::create(&npath)?;
            std::io::copy(&mut (&mut src).take(len), &mut out)?;
            Ok(())
        })();
        if copied.is_err() {
            let _ = fs::remove_file(&npath);
            return Err(err(503, "storage_error", "Storage is unavailable.", "Try again later."));
        }
        let parsed = File::open(&npath).ok().and_then(|mut f| Index::parse(&mut std::io::BufReader::new(&mut f), &self.cfg.limits).ok());
        let Some(nidx) = parsed else {
            let _ = fs::remove_file(&npath);
            return Err(err(422, "nested_invalid", "The nested archive could not be read.", "Download the entry bytes and inspect them as a separate file."));
        };
        let entries = nidx.len() as u64;
        let mut g = self.inner.lock().unwrap();
        if !g.ws.contains_key(&wsid) {
            let _ = fs::remove_file(&npath);
            return Err(not_found());
        }
        g.vers.insert(nid.clone(), Ver { id: nid.clone(), ws: wsid.clone(), owner: sid.into(), path: npath, size: len, filename, state: VState::Ready, entries, origin: "nested", created: now_unix() });
        if let Some(w) = g.ws.get_mut(&wsid) {
            w.versions.push(nid.clone());
        }
        g.cache.push((nid.clone(), Arc::new(nidx)));
        while g.cache.len() > CACHE_CAP {
            g.cache.remove(0);
        }
        Ok(Resp::json(201, format!("{{\"version_id\":{},\"entry_count\":{}}}", q(&nid), entries)))
    }

    fn entry_bytes(&self, req: &Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let basis = req.query.get("basis").map(|s| s.as_str()).unwrap_or("stored");
        match basis {
            "stored" | "uncompressed" => {}
            _ => return Err(err(422, "basis_not_supported", "Only basis=stored and basis=uncompressed are supported.", "Request basis=stored or basis=uncompressed.")),
        }
        let info = idx.info(id).ok_or_else(not_found)?;
        if info.kind == "directory" {
            return Err(err(422, "not_a_file", "Folders have no bytes.", "Choose a file."));
        }
        if info.large_resource {
            return Err(err(422, "large_resource_unsupported", "This resource uses the large-size marker, which is not supported yet.", ""));
        }
        let (off, len) = idx.stored_span(id).ok_or_else(not_found)?;
        if basis == "uncompressed" && info.kind == "file" && info.compressed {
            // Decompress in memory (bounded), then serve the requested range of the result. Plain (uncompressed) files fall through to the normal path.
            if info.encryption_tag != Some(0) {
                return Err(err(422, "entry_encrypted", "This file is marked as encrypted, so it cannot be decompressed.", "Download the stored bytes instead."));
            }
            let declared = info.uncompressed_size;
            if declared > self.cfg.max_blob || len > self.cfg.max_blob {
                return Err(err(422, "too_large_to_decompress", "This file is too large to decompress here.", "Download the stored bytes instead."));
            }
            let mut f = File::open(&path).map_err(|_| not_found())?;
            f.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
            let mut stored = vec![0u8; len as usize];
            f.read_exact(&mut stored).map_err(|_| err(422, "entry_unreadable", "The file's data could not be read from the archive.", ""))?;
            let plain = rpf_core::deflate::inflate(&stored, declared as usize)
                .map_err(|e| err(422, "inflate_failed", e.to_string(), "The entry may use a different compression. Download the stored bytes instead."))?;
            if plain.len() as u64 != declared {
                return Err(err(422, "inflate_size_mismatch", "The decompressed size does not match the size in the archive.", "Download the stored bytes instead."));
            }
            let (start, n, status, cr) = range_of(req, plain.len() as u64, true)?;
            let part: Vec<u8> = plain[start as usize..(start + n) as usize].to_vec();
            let mut r = Resp { status, headers: vec![("Accept-Ranges".into(), "bytes".into())], body: Body::Bytes { data: Arc::new(part), ctype: "application/octet-stream" } };
            if let Some(c) = cr { r.headers.push(("Content-Range".into(), c)); }
            return Ok(r);
        }
        let (start, n, status, cr) = range_of(req, len, true)?;
        let mut r = Resp { status, headers: vec![("Accept-Ranges".into(), "bytes".into())], body: if n == 0 { Body::Empty } else { Body::File { path, start: off + start, len: n } } };
        if let Some(c) = cr { r.headers.push(("Content-Range".into(), c)); }
        Ok(r)
    }
}

fn parse_id(idx: &Index, s: &str) -> Result<u32, ApiErr> {
    let id: u32 = s.parse().map_err(|_| not_found())?;
    if (id as usize) < idx.len() { Ok(id) } else { Err(not_found()) }
}

fn safe_filename(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' }).collect();
    if t.is_empty() || t.starts_with('.') { "archive.rpf".into() } else { t.chars().take(100).collect() }
}

/// Single-range handling. `limited`: payload reads (max 1 MiB per range; Range required above 1 MiB).
fn range_of(req: &Req, total: u64, limited: bool) -> Result<(u64, u64, u16, Option<String>), ApiErr> {
    let unsat = |m: &str| { let mut e = err(416, "range_not_satisfiable", m, "Send a valid single Range header."); e.details = Some(format!("{{\"size\":{}}}", total)); e };
    match req.headers.get("range") {
        None => {
            if limited && total > MAX_RANGE {
                let mut e = err(416, "range_required", "This entry is larger than 1 MiB, so a Range header is required.", "Request up to 1 MiB at a time, for example Range: bytes=0-1048575.");
                e.details = Some(format!("{{\"size\":{}}}", total));
                return Err(e);
            }
            Ok((0, total, 200, None))
        }
        Some(h) => {
            let r = h.strip_prefix("bytes=").ok_or_else(|| unsat("The Range header must start with bytes=."))?;
            if r.contains(',') {
                return Err(unsat("Multiple ranges are not supported."));
            }
            let (a, b) = r.split_once('-').ok_or_else(|| unsat("The Range header is not valid."))?;
            let a: u64 = a.trim().parse().map_err(|_| unsat("Suffix ranges are not supported. Give a start and end."))?;
            if total == 0 || a >= total {
                return Err(unsat("The range starts after the end of the data."));
            }
            let b: u64 = if b.trim().is_empty() { total - 1 } else { b.trim().parse().map_err(|_| unsat("The Range header is not valid."))? };
            if b < a {
                return Err(unsat("The range end is before its start."));
            }
            let b = b.min(total - 1);
            let n = b - a + 1;
            if limited && n > MAX_RANGE {
                return Err(unsat("A single range can be at most 1 MiB."));
            }
            Ok((a, n, 206, Some(format!("bytes {}-{}/{}", a, b, total))))
        }
    }
}
