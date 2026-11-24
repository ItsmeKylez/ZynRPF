mod api;
mod http;
mod json;
mod sha256;

use api::{Cfg, State};
use rpf_core::Limits;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

type WebFiles = std::collections::HashMap<String, (Arc<Vec<u8>>, &'static str)>;

/// Loads the built web UI into memory (flat walk, depth 3, regular files only, 4 MiB per file cap).
fn load_web(dir: String) -> WebFiles {
    fn walk(base: &std::path::Path, cur: &std::path::Path, depth: u32, out: &mut WebFiles) {
        let Ok(rd) = std::fs::read_dir(cur) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(md) = std::fs::symlink_metadata(&p) else { continue };
            if md.is_dir() && depth < 3 {
                walk(base, &p, depth + 1, out);
            } else if md.is_file() && md.len() <= 4 << 20 {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                let ctype = match p.extension().and_then(|x| x.to_str()) {
                    Some("html") => "text/html; charset=utf-8",
                    Some("js") => "text/javascript; charset=utf-8",
                    Some("css") => "text/css; charset=utf-8",
                    Some("svg") => "image/svg+xml",
                    Some("json") => "application/json; charset=utf-8",
                    Some("woff2") => "font/woff2",
                    _ => continue,
                };
                if let Ok(b) = std::fs::read(&p) {
                    out.insert(format!("/{}", rel), (Arc::new(b), ctype));
                }
            }
        }
    }
    let mut m = WebFiles::new();
    let base = PathBuf::from(dir);
    walk(&base, &base, 0, &mut m);
    m
}

fn main() {
    // Defaults are SUGGESTED starting values (D-029), not measured.
    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or("./data".into()));
    // Anonymous workspaces do not survive a restart in M1 (sessions live in memory), so stale data is cleared.
    // Clear the contents, not the directory itself: DATA_DIR may be a mounted volume that cannot be removed.
    std::fs::create_dir_all(&data_dir).expect("cannot create DATA_DIR");
    if let Ok(rd) = std::fs::read_dir(&data_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
    }
    let secure = std::env::var("COOKIE_SECURE").map(|v| v == "true" || v == "1").unwrap_or(false);
    let cfg = Cfg {
        data_dir,
        max_upload: env_u64("MAX_UPLOAD_BYTES", 1 << 30),
        quota: env_u64("SESSION_QUOTA_BYTES", 2 << 30),
        global_quota: env_u64("GLOBAL_QUOTA_BYTES", 0),
        chunk: env_u64("CHUNK_SIZE_BYTES", 8 << 20),
        idle: Duration::from_secs(env_u64("IDLE_SECONDS", 86_400)),
        upload_ttl: Duration::from_secs(env_u64("UPLOAD_TTL_SECONDS", 86_400)),
        secure,
        limits: Limits { max_entries: env_u64("MAX_ENTRIES", 1_000_000) as u32, max_names_bytes: 65_536, max_depth: 128 },
        max_workspaces: env_u64("MAX_WORKSPACES", 1) as usize,
        max_sessions: env_u64("MAX_SESSIONS", 10_000) as usize,
        rate_reads: env_u64("RATE_READS_PER_MIN", 600) as u32,
        rate_writes: env_u64("RATE_WRITES_PER_MIN", 120) as u32,
        max_blob: env_u64("MAX_BLOB_BYTES", 256 << 20),
    };
    let max_conns = env_u64("MAX_CONNECTIONS", 256) as usize;
    let port = std::env::var("PORT").unwrap_or("8080".into());
    let web = load_web(std::env::var("WEB_DIR").unwrap_or("./web/dist".into()));
    println!("web UI files loaded: {}", web.len());
    let st = Arc::new(State::new(cfg, web));
    {
        let st = st.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(30));
            st.janitor();
        });
    }
    let l = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("cannot bind port");
    println!("rpf-editor server listening on {}", port);
    for c in l.incoming().flatten() {
        let id = st.request_id();
        if st.conns.fetch_add(1, Ordering::SeqCst) >= max_conns {
            st.conns.fetch_sub(1, Ordering::SeqCst);
            let mut c = c;
            let e = http::err(503, "busy", "The server is busy.", "Try again in a moment.");
            http::write_response(&mut c, http::err_resp(&e, &id), &id, secure);
            continue;
        }
        let st = st.clone();
        std::thread::spawn(move || {
            match http::read_request(c, id.clone()) {
                Ok(mut req) => {
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| st.route(&mut req)));
                    let resp = match res {
                        Ok(Ok(r)) => r,
                        Ok(Err(e)) => http::err_resp(&e, &id),
                        Err(_) => http::err_resp(&http::err(500, "internal_error", "Something went wrong on the server.", "Try again. If it keeps happening, report the request ID."), &id),
                    };
                    http::write_response(&mut req.stream, resp, &id, st.cfg.secure);
                }
                Err((mut s, e)) => http::write_response(&mut s, http::err_resp(&e, &id), &id, st.cfg.secure),
            }
            st.conns.fetch_sub(1, Ordering::SeqCst);
        });
    }
}
