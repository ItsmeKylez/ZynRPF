//! M4 route tests: edit/save/undo/import/export/backups through the real router, synthetic archives from the real core.
use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode, header},
};
use http_body_util::BodyExt;
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


use std::io::Read;

fn jreq(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json").body(Body::from(body.to_string())).unwrap()
}

fn jreq_im(method: &str, uri: &str, body: serde_json::Value, if_match: u64) -> Request<Body> {
    Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json").header(header::IF_MATCH, if_match.to_string()).body(Body::from(body.to_string())).unwrap()
}

fn multi(uri: &str, parts: &[(&str, &[u8])]) -> Request<Body> {
    let mut b = Vec::new();
    for (name, data) in parts {
        b.extend_from_slice(format!("--XB\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        b.extend_from_slice(data);
        b.extend_from_slice(b"\r\n");
    }
    b.extend_from_slice(b"--XB--\r\n");
    Request::post(uri).header(header::CONTENT_TYPE, "multipart/form-data; boundary=XB").body(Body::from(b)).unwrap()
}

fn paths(v: &serde_json::Value) -> Vec<String> {
    fn walk(n: &serde_json::Value, out: &mut Vec<String>) {
        for c in n.as_array().into_iter().flatten() {
            out.push(c["path"].as_str().unwrap().to_string());
            walk(&c["children"], out);
        }
    }
    let mut out = Vec::new();
    walk(&v["tree"], &mut out);
    out
}

async fn setup(e: &Env) -> (String, String, u64, String) {
    let (cookie, _) = new_session(e).await;
    let (st, up) = upload(e, &cookie, "mod.rpf", &sample_rpf()).await;
    assert_eq!(st, StatusCode::OK);
    let file_id = up[0]["fileId"].as_str().unwrap().to_string();
    let r = send(e, open_req(&file_id), Some(&cookie), true).await;
    let s = json(r).await;
    (cookie, file_id, s["docId"].as_u64().unwrap(), s["revision"].to_string())
}

#[tokio::test]
async fn edit_undo_redo_save_and_backups() {
    let e = env(|_| {});
    let (c, fid, id, _) = setup(&e).await;
    let d = format!("/api/v1/documents/{id}");

    // create text, edit, mkdir, rename, move
    let r = send(&e, jreq("POST", &format!("{d}/entries/text"), serde_json::json!({"path":"content/new.txt","text":"one"})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let s = json(r).await;
    assert!(s["dirty"].as_bool().unwrap() && s["undoDepth"] == 1);
    assert!(paths(&s).contains(&"content/new.txt".to_string()));
    let r = send(&e, jreq("PUT", &format!("{d}/entries/text"), serde_json::json!({"path":"content/new.txt","text":"two"})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let v = json(send(&e, get(&format!("{d}/entries?path=content/new.txt")), Some(&c), false).await).await;
    assert_eq!((v["kind"].as_str().unwrap(), v["text"].as_str().unwrap()), ("text", "two"));
    for (route, b) in [
        ("mkdir", serde_json::json!({"path":"content/d"})),
        ("rename", serde_json::json!({"path":"content/new.txt","newName":"renamed.txt"})),
        ("move", serde_json::json!({"paths":["content/renamed.txt"],"dir":"content/d"})),
    ] {
        let r = send(&e, jreq("POST", &format!("{d}/{route}"), b), Some(&c), true).await;
        assert_eq!(r.status(), StatusCode::OK, "{route}");
    }
    let s = json(send(&e, get(&d), Some(&c), false).await).await;
    assert!(paths(&s).contains(&"content/d/renamed.txt".to_string()));

    // undo / redo
    let s = json(send(&e, Request::post(format!("{d}/undo")).body(Body::empty()).unwrap(), Some(&c), true).await).await;
    assert!(!paths(&s).contains(&"content/d/renamed.txt".to_string()) && s["redoDepth"] == 1);
    let s = json(send(&e, Request::post(format!("{d}/redo")).body(Body::empty()).unwrap(), Some(&c), true).await).await;
    assert!(paths(&s).contains(&"content/d/renamed.txt".to_string()));
    let r = send(&e, Request::post(format!("{d}/redo")).body(Body::empty()).unwrap(), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert_eq!(json(r).await["error"]["code"], "NOTHING_TO_REDO");

    // first save: no backup yet (original is the preserved previous version)
    let r = send(&e, Request::post(format!("{d}/save")).body(Body::empty()).unwrap(), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let sv = json(r).await;
    assert_eq!(sv["summary"]["dirty"], false);
    assert!(sv["backupId"].is_null());
    assert!(sv["newSize"].as_u64().unwrap() > 0);

    // original untouched, current has the edit
    let orig = body(send(&e, get(&format!("/api/v1/files/{fid}/download?version=original")), Some(&c), false).await).await;
    assert_eq!(orig, sample_rpf());
    let cur = body(send(&e, get(&format!("/api/v1/files/{fid}/download?version=current")), Some(&c), false).await).await;
    assert_ne!(cur, orig);
    assert!(send(&e, get(&format!("/api/v1/files/{fid}/download?version=bogus")), Some(&c), false).await.status() == StatusCode::BAD_REQUEST);

    // close + reopen shows the saved state
    assert_eq!(send(&e, Request::delete(&d).body(Body::empty()).unwrap(), Some(&c), true).await.status(), StatusCode::NO_CONTENT);
    let s = json(send(&e, open_req(&fid), Some(&c), true).await).await;
    assert!(paths(&s).contains(&"content/d/renamed.txt".to_string()));
    let id2 = s["docId"].as_u64().unwrap();
    let d2 = format!("/api/v1/documents/{id2}");

    // second save creates a backup of the first saved version
    send(&e, jreq("POST", &format!("{d2}/delete"), serde_json::json!({"paths":["content/d"]})), Some(&c), true).await;
    let sv = json(send(&e, Request::post(format!("{d2}/save")).body(Body::empty()).unwrap(), Some(&c), true).await).await;
    let bid = sv["backupId"].as_str().unwrap().to_string();
    let list = json(send(&e, get(&format!("/api/v1/files/{fid}/backups")), Some(&c), false).await).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["backupId"], bid.as_str());
    let bk = body(send(&e, get(&format!("/api/v1/files/{fid}/backups/{bid}/download")), Some(&c), false).await).await;
    assert_eq!(bk, cur, "backup is exactly the previously saved version");

    // restore brings the folder back and clears history
    let s = json(send(&e, jreq("POST", &format!("{d2}/restore"), serde_json::json!({"backupId": bid})), Some(&c), true).await).await;
    assert!(paths(&s).contains(&"content/d/renamed.txt".to_string()));
    assert_eq!((s["dirty"].as_bool().unwrap(), s["undoDepth"].as_u64().unwrap()), (false, 0));
    // forged backup ids are plain 404s
    let r = send(&e, jreq("POST", &format!("{d2}/restore"), serde_json::json!({"backupId": "../current"})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn import_replace_export_zip() {
    let e = env(|_| {});
    let (c, _, id, _) = setup(&e).await;
    let d = format!("/api/v1/documents/{id}");
    let r = send(&e, multi(&format!("{d}/import?dir=content"), &[("skins/a/b.txt", b"bb"), ("c.bin", &[9, 9])]), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let s = json(r).await;
    assert!(paths(&s).contains(&"content/skins/a/b.txt".to_string()) && s["undoDepth"] == 1);
    // hostile relative path rejected, nothing changes
    let r = send(&e, multi(&format!("{d}/import"), &[("../evil.txt", b"x")]), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(send(&e, get(&d), Some(&c), false).await).await["undoDepth"], 1);

    let r = send(&e, multi(&format!("{d}/replace?path=content/c.bin"), &[("whatever.bin", &[7])]), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let raw = body(send(&e, get(&format!("{d}/entries/raw?path=content/c.bin")), Some(&c), false).await).await;
    assert_eq!(raw, vec![7]);
    // two parts is not a replace
    let r = send(&e, multi(&format!("{d}/replace?path=content/c.bin"), &[("a", b"1"), ("b", b"2")]), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);

    let r = send(&e, jreq("POST", &format!("{d}/export"), serde_json::json!({"paths":["content"]})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()[header::CONTENT_TYPE], "application/zip");
    assert!(r.headers()["x-zyn-file-count"].to_str().unwrap().parse::<u64>().unwrap() >= 2);
    assert!(r.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("mod.zip"));
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(body(r).await)).unwrap();
    let mut t = String::new();
    z.by_name("content/skins/a/b.txt").unwrap().read_to_string(&mut t).unwrap();
    assert_eq!(t, "bb");
    let r = send(&e, jreq("POST", &format!("{d}/export"), serde_json::json!({"paths":[]})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stale_if_match_is_412_and_changes_nothing() {
    let e = env(|_| {});
    let (c, _, id, rev) = setup(&e).await;
    let d = format!("/api/v1/documents/{id}");
    let rev: u64 = rev.parse().unwrap();
    let r = send(&e, jreq_im("POST", &format!("{d}/mkdir"), serde_json::json!({"path":"a"}), rev), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let newrev = json(r).await["revision"].as_u64().unwrap();
    assert_ne!(newrev, rev);
    let r = send(&e, jreq_im("POST", &format!("{d}/mkdir"), serde_json::json!({"path":"b"}), rev), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::PRECONDITION_FAILED);
    assert_eq!(json(r).await["error"]["code"], "STALE_DOCUMENT");
    let r = send(&e, Request::post(format!("{d}/save")).header(header::IF_MATCH, rev.to_string()).body(Body::empty()).unwrap(), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::PRECONDITION_FAILED);
    let r = send(&e, Request::post(format!("{d}/mkdir")).header(header::CONTENT_TYPE, "application/json").header(header::IF_MATCH, "abc").body(Body::from("{\"path\":\"c\"}")).unwrap(), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(send(&e, get(&d), Some(&c), false).await).await["undoDepth"], 1);
}

#[tokio::test]
async fn validation_errors_are_stable_and_path_free() {
    let e = env(|_| {});
    let (c, _, id, _) = setup(&e).await;
    let d = format!("/api/v1/documents/{id}");
    let cases = [
        ("rename", serde_json::json!({"path":"content/readme.txt","newName":"a/b"}), StatusCode::BAD_REQUEST, "INVALID_NAME"),
        ("rename", serde_json::json!({"path":"content/nope","newName":"x"}), StatusCode::NOT_FOUND, "NOT_FOUND"),
        ("mkdir", serde_json::json!({"path":"../x"}), StatusCode::BAD_REQUEST, "INVALID_PATH"),
        ("move", serde_json::json!({"paths":["content"],"dir":"content"}), StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
        ("delete", serde_json::json!({"paths":["missing"]}), StatusCode::NOT_FOUND, "NOT_FOUND"),
    ];
    for (route, b, status, code) in cases {
        let r = send(&e, jreq("POST", &format!("{d}/{route}"), b), Some(&c), true).await;
        assert_eq!(r.status(), status, "{route}");
        let j = json(r).await;
        assert_eq!(j["error"]["code"], code, "{route}");
        let msg = j["error"]["message"].as_str().unwrap();
        assert!(!msg.contains(&e.root.to_string_lossy().to_string()) && !msg.contains('/') , "{msg}");
    }
    let r = send(&e, Request::post(format!("{d}/undo")).body(Body::empty()).unwrap(), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::CONFLICT);
    // state-changing routes need the CSRF header
    let r = send(&e, jreq("POST", &format!("{d}/mkdir"), serde_json::json!({"path":"x"})), Some(&c), false).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_eq!(json(send(&e, get(&d), Some(&c), false).await).await["undoDepth"], 0);
}

#[tokio::test]
async fn other_sessions_cannot_touch_documents_or_backups() {
    let e = env(|_| {});
    let (a, fid, id, _) = setup(&e).await;
    let d = format!("/api/v1/documents/{id}");
    send(&e, Request::post(format!("{d}/save")).body(Body::empty()).unwrap(), Some(&a), true).await;
    let (b, _) = new_session(&e).await;
    for req in [
        jreq("POST", &format!("{d}/mkdir"), serde_json::json!({"path":"x"})),
        Request::post(format!("{d}/save")).body(Body::empty()).unwrap(),
        Request::post(format!("{d}/undo")).body(Body::empty()).unwrap(),
        jreq("POST", &format!("{d}/export"), serde_json::json!({"paths":[""]})),
        jreq("POST", &format!("{d}/restore"), serde_json::json!({"backupId":"000001"})),
    ] {
        assert_eq!(send(&e, req, Some(&b), true).await.status(), StatusCode::NOT_FOUND);
    }
    for u in [format!("/api/v1/files/{fid}/backups"), format!("/api/v1/files/{fid}/download?version=current"), format!("{d}/entries?path=content/readme.txt")] {
        assert_eq!(send(&e, get(&u), Some(&b), false).await.status(), StatusCode::NOT_FOUND, "{u}");
    }
}

#[tokio::test]
async fn visual_settings_read_and_update() {
    let e = env(|_| {});
    let (c, _, id, _) = setup(&e).await;
    let d = format!("/api/v1/documents/{id}");
    let vs = "light.intensity 1.5f\r\nfog.density 0.25\r\nfog.density 0.30\r\n";
    let r = send(&e, jreq("POST", &format!("{d}/entries/text"), serde_json::json!({"path":"visualsettings.dat","text":vs})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let v = json(send(&e, get(&format!("{d}/settings?path=visualsettings.dat")), Some(&c), false).await).await;
    assert_eq!(v["entries"].as_array().unwrap().len(), 2);
    assert_eq!(v["entries"][1]["repeat"], 2);
    let r = send(&e, jreq("PUT", &format!("{d}/settings"), serde_json::json!({"path":"visualsettings.dat","values":[["light.intensity","3"],["fog.density","0.5"]]})), Some(&c), true).await;
    assert_eq!(r.status(), StatusCode::OK);
    let raw = body(send(&e, get(&format!("{d}/entries/raw?path=visualsettings.dat")), Some(&c), false).await).await;
    assert_eq!(String::from_utf8(raw).unwrap(), "light.intensity 3f\r\nfog.density 0.5\r\nfog.density 0.5\r\n");
    for (vals, status, code) in [
        (serde_json::json!([["nope","1"]]), StatusCode::NOT_FOUND, "NOT_FOUND"),
        (serde_json::json!([["light.intensity","1\nx 2"]]), StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
    ] {
        let r = send(&e, jreq("PUT", &format!("{d}/settings"), serde_json::json!({"path":"visualsettings.dat","values":vals})), Some(&c), true).await;
        assert_eq!(r.status(), status);
        assert_eq!(json(r).await["error"]["code"], code);
    }
    // another session cannot read or change them
    let (b, _) = new_session(&e).await;
    assert_eq!(send(&e, get(&format!("{d}/settings?path=visualsettings.dat")), Some(&b), false).await.status(), StatusCode::NOT_FOUND);
}
