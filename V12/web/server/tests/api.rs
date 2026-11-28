//! Route tests against the real router (no network socket) with synthetic RPF archives built by the real core.
use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode, header},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tlk_rpf_core::rpf::{Arsiv, Dosya, Dugum};
use tower::ServiceExt;
use zyn_server::{app, config::Config, store::WorkspaceStore};

fn sample_rpf() -> Vec<u8> {
    let mut a = Arsiv::yeni();
    a.dizin_olustur("content").unwrap();
    a.koy("content", Dugum::Dosya(Dosya::yeni("readme.txt", b"hello".to_vec()))).unwrap();
    a.yaz().unwrap()
}

struct Env {
    router: Router,
    root: std::path::PathBuf,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn env(tweak: impl FnOnce(&mut Config)) -> Env {
    let root = std::env::temp_dir().join(format!("zyn-api-test-{}-{:?}", std::process::id(), std::time::Instant::now()));
    let mut cfg = Config { workspace_root: root.clone(), cookie_secure: false, ..Config::default() };
    tweak(&mut cfg);
    let state = app::AppState::new(WorkspaceStore::new(cfg).unwrap());
    Env { router: app::router(state), root }
}

async fn send(e: &Env, mut req: Request<Body>, cookie: Option<&str>, csrf: bool) -> Response<Body> {
    if let Some(c) = cookie {
        req.headers_mut().insert(header::COOKIE, c.parse().unwrap());
    }
    if csrf {
        req.headers_mut().insert("x-requested-with", "zyn".parse().unwrap());
    }
    e.router.clone().oneshot(req).await.unwrap()
}

async fn body(r: Response<Body>) -> Vec<u8> {
    r.into_body().collect().await.unwrap().to_bytes().to_vec()
}

async fn json(r: Response<Body>) -> serde_json::Value {
    serde_json::from_slice(&body(r).await).unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

/// Starts a session and returns the `zyn_session=<token>` cookie pair.
async fn new_session(e: &Env) -> (String, String) {
    let r = send(e, get("/api/v1/session"), None, false).await;
    assert_eq!(r.status(), StatusCode::OK);
    let set = r.headers().get(header::SET_COOKIE).expect("Set-Cookie").to_str().unwrap().to_string();
    let pair = set.split(';').next().unwrap().to_string();
    (pair, set)
}

fn multipart(name: &str, data: &[u8]) -> Request<Body> {
    let mut b = format!("--XB\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").into_bytes();
    b.extend_from_slice(data);
    b.extend_from_slice(b"\r\n--XB--\r\n");
    Request::post("/api/v1/files").header(header::CONTENT_TYPE, "multipart/form-data; boundary=XB").body(Body::from(b)).unwrap()
}

async fn upload(e: &Env, cookie: &str, name: &str, data: &[u8]) -> (StatusCode, serde_json::Value) {
    let r = send(e, multipart(name, data), Some(cookie), true).await;
    (r.status(), json(r).await)
}

fn open_req(file_id: &str) -> Request<Body> {
    Request::post("/api/v1/documents").header(header::CONTENT_TYPE, "application/json").body(Body::from(format!("{{\"fileId\":\"{file_id}\"}}"))).unwrap()
}

#[tokio::test]
async fn health_endpoints_need_no_session() {
    let e = env(|_| {});
    for p in ["/healthz", "/readyz"] {
        let r = send(&e, get(p), None, false).await;
        assert_eq!(r.status(), StatusCode::OK, "{p}");
        assert!(r.headers().get(header::SET_COOKIE).is_none());
    }
}

#[tokio::test]
async fn dev_harness_is_served_with_strict_csp() {
    let e = env(|_| {});
    let r = send(&e, get("/"), None, false).await;
    assert_eq!(r.status(), StatusCode::OK);
    let csp = r.headers()[header::CONTENT_SECURITY_POLICY].to_str().unwrap().to_string();
    assert!(csp.contains("script-src 'self'") && csp.contains("default-src 'none'") && !csp.contains("script-src 'unsafe-inline'"), "{csp}");
    let html = String::from_utf8(body(r).await).unwrap();
    assert!(html.contains("M2 dev harness") && html.contains("/dev.js"));
    let js = send(&e, get("/dev.js"), None, false).await;
    assert_eq!(js.status(), StatusCode::OK);
    assert!(js.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/javascript"));
}

#[tokio::test]
async fn session_cookie_is_hardened() {
    let e = env(|c| c.cookie_secure = true);
    let (_, set) = new_session(&e).await;
    for flag in ["HttpOnly", "SameSite=Strict", "Path=/", "Secure"] {
        assert!(set.contains(flag), "{set}");
    }
}

#[tokio::test]
async fn full_slice_upload_open_tree_entry_export() {
    let e = env(|_| {});
    let (cookie, _) = new_session(&e).await;
    let original = sample_rpf();

    let (st, files) = upload(&e, &cookie, "mod.rpf", &original).await;
    assert_eq!(st, StatusCode::OK, "{files}");
    let file_id = files[0]["fileId"].as_str().unwrap().to_string();
    assert_eq!(files[0]["name"], "mod.rpf");
    assert_eq!(files[0]["size"], original.len());

    // download returns the untouched upload
    let r = send(&e, get(&format!("/api/v1/files/{file_id}/download")), Some(&cookie), false).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(r.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("mod.rpf"));
    assert_eq!(body(r).await, original);

    // open through the real core; tree is shown
    let r = send(&e, open_req(&file_id), Some(&cookie), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let s = json(r).await;
    assert_eq!(s["docId"], 1);
    assert_eq!(s["encryption"], "OPEN");
    assert_eq!(s["tree"][0]["children"][0]["path"], "content/readme.txt");

    // opening again is idempotent
    let again = json(send(&e, open_req(&file_id), Some(&cookie), true).await).await;
    assert_eq!(again["docId"], 1);

    // single entry download
    let r = send(&e, get("/api/v1/documents/1/entry?path=content%2Freadme.txt"), Some(&cookie), false).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await, b"hello");

    // verified re-serialized archive is a valid RPF containing the entry
    let r = send(&e, get("/api/v1/documents/1/export"), Some(&cookie), false).await;
    assert_eq!(r.status(), StatusCode::OK);
    let exported = body(r).await;
    let re = Arsiv::oku(Arc::from(exported)).expect("exported archive re-parses");
    assert_eq!(re.dosya("content/readme.txt").unwrap().veri().unwrap().as_ref(), b"hello");

    // list + close
    let list = json(send(&e, get("/api/v1/documents"), Some(&cookie), false).await).await;
    assert_eq!(list[0]["fileId"], file_id.as_str());
    let r = send(&e, Request::delete("/api/v1/documents/1").body(Body::empty()).unwrap(), Some(&cookie), true).await;
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(send(&e, get("/api/v1/documents/1"), Some(&cookie), false).await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn workspaces_are_isolated() {
    let e = env(|_| {});
    let (a, _) = new_session(&e).await;
    let (b, _) = new_session(&e).await;
    assert_ne!(a, b);
    let (_, files) = upload(&e, &a, "a.rpf", &sample_rpf()).await;
    let file_id = files[0]["fileId"].as_str().unwrap();
    assert_eq!(send(&e, open_req(file_id), Some(&a), true).await.status(), StatusCode::OK);

    // B cannot open, download, read documents or entries from A.
    assert_eq!(send(&e, open_req(file_id), Some(&b), true).await.status(), StatusCode::NOT_FOUND);
    assert_eq!(send(&e, get(&format!("/api/v1/files/{file_id}/download")), Some(&b), false).await.status(), StatusCode::NOT_FOUND);
    assert_eq!(send(&e, get("/api/v1/documents/1"), Some(&b), false).await.status(), StatusCode::NOT_FOUND);
    assert_eq!(send(&e, get("/api/v1/documents/1/export"), Some(&b), false).await.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(send(&e, get("/api/v1/files"), Some(&b), false).await).await, serde_json::json!([]));
    // A forged/unknown cookie gets a fresh empty workspace, not someone else's.
    let forged = format!("zyn_session={}", "a".repeat(64));
    assert_eq!(json(send(&e, get("/api/v1/files"), Some(&forged), false).await).await, serde_json::json!([]));
}

#[tokio::test]
async fn csrf_header_required_for_mutations() {
    let e = env(|_| {});
    let (cookie, _) = new_session(&e).await;
    let r = send(&e, multipart("a.rpf", &sample_rpf()), Some(&cookie), false).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_eq!(json(r).await["error"]["code"], "CSRF_REJECTED");
}

#[tokio::test]
async fn upload_limit_is_enforced_and_leaves_nothing() {
    let e = env(|c| c.max_upload_bytes = 100);
    let (cookie, _) = new_session(&e).await;
    let (st, j) = upload(&e, &cookie, "big.rpf", &vec![7u8; 5000]).await;
    assert_eq!(st, StatusCode::PAYLOAD_TOO_LARGE, "{j}");
    assert_eq!(j["error"]["code"], "UPLOAD_TOO_LARGE");
    assert_eq!(json(send(&e, get("/api/v1/files"), Some(&cookie), false).await).await, serde_json::json!([]));
}

#[tokio::test]
async fn invalid_rpf_gets_stable_error_without_paths() {
    let e = env(|_| {});
    let (cookie, _) = new_session(&e).await;
    let (_, files) = upload(&e, &cookie, "junk.rpf", b"this is not an archive").await;
    let r = send(&e, open_req(files[0]["fileId"].as_str().unwrap()), Some(&cookie), true).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json(r).await;
    assert!(["RPF_INVALID", "RPF_CORRUPT"].contains(&j["error"]["code"].as_str().unwrap()), "{j}");
    let text = j.to_string();
    assert!(!text.contains(e.root.to_str().unwrap()) && !text.contains("/tmp"), "{text}");
}

#[tokio::test]
async fn entry_path_validation_and_not_found() {
    let e = env(|_| {});
    let (cookie, _) = new_session(&e).await;
    let (_, files) = upload(&e, &cookie, "m.rpf", &sample_rpf()).await;
    send(&e, open_req(files[0]["fileId"].as_str().unwrap()), Some(&cookie), true).await;
    for (q, code, status) in [
        ("..%2F..%2Fetc%2Fpasswd", "INVALID_PATH", StatusCode::BAD_REQUEST),
        ("content%2Fnope.txt", "NOT_FOUND", StatusCode::NOT_FOUND),
        ("content%5Creadme.txt", "INVALID_PATH", StatusCode::BAD_REQUEST),
    ] {
        let r = send(&e, get(&format!("/api/v1/documents/1/entry?path={q}")), Some(&cookie), false).await;
        assert_eq!(r.status(), status, "{q}");
        assert_eq!(json(r).await["error"]["code"], code, "{q}");
    }
}

#[tokio::test]
async fn delete_session_removes_everything() {
    let e = env(|_| {});
    let (cookie, _) = new_session(&e).await;
    let (_, files) = upload(&e, &cookie, "m.rpf", &sample_rpf()).await;
    let id = files[0]["fileId"].as_str().unwrap().to_string();
    let r = send(&e, Request::delete("/api/v1/session").body(Body::empty()).unwrap(), Some(&cookie), true).await;
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    // old cookie now yields a fresh, empty workspace
    assert_eq!(send(&e, get(&format!("/api/v1/files/{id}/download")), Some(&cookie), false).await.status(), StatusCode::NOT_FOUND);
    let dirs = std::fs::read_dir(&e.root).unwrap().count();
    assert!(dirs <= 1, "only the fresh empty workspace may remain, found {dirs}");
}

#[tokio::test]
async fn static_frontend_is_served_with_spa_fallback_and_json_api_404() {
    let dir = std::env::temp_dir().join(format!("zyn-static-{}-{:?}", std::process::id(), std::time::Instant::now()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><title>spa-index</title>").unwrap();
    std::fs::write(dir.join("assets/app.js"), "console.log('app')").unwrap();
    let static_dir = dir.clone();
    let e = env(move |c| c.static_dir = Some(static_dir));

    let index = |r: Response<Body>| async move { String::from_utf8(body(r).await).unwrap() };
    let r = send(&e, get("/"), None, false).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(index(r).await.contains("spa-index"));

    let r = send(&e, get("/assets/app.js"), None, false).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(index(r).await, "console.log('app')");

    // A client-side route (reload on a deep link) falls back to the page...
    let r = send(&e, get("/workspace/some/route"), None, false).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(index(r).await.contains("spa-index"));

    // ...but an unknown API path is a JSON 404, never HTML.
    let r = send(&e, get("/api/v1/nope"), None, false).await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(r).await["error"]["code"], "NOT_FOUND");

    // The developer harness is still reachable, at /dev.
    let r = send(&e, get("/dev"), None, false).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(index(r).await.contains("M2 dev harness"));
    let _ = std::fs::remove_dir_all(dir);
}
