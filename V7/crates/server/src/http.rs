//! Minimal HTTP/1.1 layer (std only): one request per connection, strict size limits, timeouts.
//! TEMPORARY: replace with a maintained HTTP framework once crates can be fetched; the API layer is independent of it.
use crate::json::q;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

pub struct ApiErr {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
    pub next: Option<String>,
    pub details: Option<String>,
    pub retry_after: Option<u64>,
}
pub fn err(status: u16, code: &'static str, message: impl Into<String>, next: &str) -> ApiErr {
    ApiErr { status, code, message: message.into(), next: if next.is_empty() { None } else { Some(next.into()) }, details: None, retry_after: None }
}
pub fn not_found() -> ApiErr {
    err(404, "not_found", "That item was not found.", "Check the address, or reload the page.")
}

pub enum Body {
    Json(String),
    Empty,
    Bytes { data: std::sync::Arc<Vec<u8>>, ctype: &'static str },
    File { path: PathBuf, start: u64, len: u64 },
}
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}
impl Resp {
    pub fn json(status: u16, body: String) -> Resp {
        Resp { status, headers: vec![], body: Body::Json(body) }
    }
    pub fn empty(status: u16) -> Resp {
        Resp { status, headers: vec![], body: Body::Empty }
    }
}

pub struct Req {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub id: String,
    pub peer: String,
    pub content_length: u64,
    pub stream: TcpStream,
    pre: Vec<u8>,
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => o.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = |c: u8| (c as char).to_digit(16);
                if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                    o.push((h * 16 + l) as u8);
                    i += 2;
                } else {
                    o.push(b'%');
                }
            }
            c => o.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&o).to_string()
}

impl Req {
    pub fn cookie(&self, name: &str) -> Option<String> {
        let c = self.headers.get("cookie")?;
        c.split(';').find_map(|p| {
            let (k, v) = p.trim().split_once('=')?;
            if k == name { Some(v.to_string()) } else { None }
        })
    }
    /// Reads the whole (small) body.
    pub fn body(&mut self, max: u64) -> Result<Vec<u8>, ApiErr> {
        if self.content_length > max {
            return Err(err(413, "payload_too_large", "The request body is too large.", "Send a smaller request."));
        }
        let mut out = Vec::with_capacity(self.content_length as usize);
        self.stream_body_to(&mut out).map_err(|_| err(400, "body_incomplete", "The request body was incomplete.", "Try again."))?;
        Ok(out)
    }
    /// Reads and discards up to `max` unread body bytes so an early error response is not lost to a connection reset.
    /// Larger bodies are left unread (the connection is closed after the response).
    pub fn drain_body(&mut self, max: u64) {
        if self.content_length == 0 || self.content_length > max {
            return;
        }
        let _ = self.stream_body_to(&mut std::io::sink());
    }
    /// Streams exactly `content_length` bytes into `w`; Err if the client stops early or `w` fails.
    pub fn stream_body_to<W: Write>(&mut self, w: &mut W) -> Result<(), ()> {
        let cl = self.content_length;
        let first = &self.pre[..self.pre.len().min(cl as usize)];
        w.write_all(first).map_err(|_| ())?;
        let mut got = first.len() as u64;
        let mut b = vec![0u8; 65536];
        while got < cl {
            let want = ((cl - got) as usize).min(b.len());
            match self.stream.read(&mut b[..want]) {
                Ok(0) | Err(_) => return Err(()),
                Ok(n) => {
                    w.write_all(&b[..n]).map_err(|_| ())?;
                    got += n as u64;
                }
            }
        }
        Ok(())
    }
}

pub fn read_request(mut stream: TcpStream, id: String) -> Result<Req, (TcpStream, ApiErr)> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
    let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let hend = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if buf.len() > 16384 {
            return Err((stream, err(431, "headers_too_large", "The request headers are too large.", "")));
        }
        match stream.read(&mut tmp) {
            Ok(0) | Err(_) => return Err((stream, err(400, "bad_request", "The request could not be read.", ""))),
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..hend]).to_string();
    let pre = buf[hend + 4..].to_vec();
    let mut lines = head.lines();
    let mut rl = lines.next().unwrap_or("").split(' ');
    let (method, target) = (rl.next().unwrap_or("").to_string(), rl.next().unwrap_or("").to_string());
    let mut headers = HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err((stream, err(400, "bad_request", "Chunked transfer encoding is not supported.", "Send a Content-Length header.")));
    }
    let content_length = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let (path, qs) = target.split_once('?').unwrap_or((&target, ""));
    let mut query = HashMap::new();
    for kv in qs.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        query.insert(pct_decode(k), pct_decode(v));
    }
    Ok(Req { method, path: pct_decode(path), query, headers, id, peer, content_length, stream, pre })
}

fn status_text(s: u16) -> &'static str {
    match s {
        200 => "OK", 201 => "Created", 202 => "Accepted", 204 => "No Content", 206 => "Partial Content",
        400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden", 404 => "Not Found", 409 => "Conflict",
        413 => "Payload Too Large", 416 => "Range Not Satisfiable", 422 => "Unprocessable Entity",
        429 => "Too Many Requests", 431 => "Request Header Fields Too Large", 503 => "Service Unavailable",
        _ => "Error",
    }
}

pub fn err_resp(e: &ApiErr, request_id: &str) -> Resp {
    let mut s = format!("{{\"error\":{{\"code\":{},\"message\":{}", q(e.code), q(&e.message));
    if let Some(n) = &e.next {
        s += &format!(",\"next_step\":{}", q(n));
    }
    if let Some(d) = &e.details {
        s += &format!(",\"details\":{}", d);
    }
    s += &format!(",\"request_id\":{}}}}}", q(request_id));
    let mut r = Resp::json(e.status, s);
    if let Some(ra) = e.retry_after {
        r.headers.push(("Retry-After".into(), ra.to_string()));
    }
    r
}

pub fn write_response(stream: &mut TcpStream, r: Resp, request_id: &str, secure: bool) {
    let (len, ctype) = match &r.body {
        Body::Json(s) => (s.len() as u64, "application/json; charset=utf-8"),
        Body::Empty => (0, "text/plain"),
        Body::Bytes { data, ctype } => (data.len() as u64, *ctype),
        Body::File { len, .. } => (*len, "application/octet-stream"),
    };
    let mut h = format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n", r.status, status_text(r.status), ctype, len);
    h += &format!("X-Request-Id: {}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: same-origin\r\n", request_id);
    h += "Content-Security-Policy: default-src 'self'; img-src 'self' blob:; frame-ancestors 'none'\r\nPermissions-Policy: camera=(), microphone=(), geolocation=()\r\n";
    if !r.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("cache-control")) {
        h += "Cache-Control: private, no-store\r\n";
    }
    if secure {
        h += "Strict-Transport-Security: max-age=31536000; includeSubDomains\r\n";
    }
    for (k, v) in &r.headers {
        // header values are server-built; refuse CR/LF defensively
        if !k.contains(['\r', '\n']) && !v.contains(['\r', '\n']) {
            h += &format!("{}: {}\r\n", k, v);
        }
    }
    h += "\r\n";
    if stream.write_all(h.as_bytes()).is_err() {
        return;
    }
    match r.body {
        Body::Json(s) => {
            let _ = stream.write_all(s.as_bytes());
        }
        Body::Empty => {}
        Body::Bytes { data, .. } => {
            let _ = stream.write_all(&data);
        }
        Body::File { path, start, len } => {
            let Ok(mut f) = File::open(&path) else { return };
            if f.seek(SeekFrom::Start(start)).is_err() {
                return;
            }
            let mut left = len;
            let mut b = vec![0u8; 65536];
            while left > 0 {
                let w = (left as usize).min(b.len());
                match f.read(&mut b[..w]) {
                    Ok(0) | Err(_) => return,
                    Ok(k) => {
                        if stream.write_all(&b[..k]).is_err() {
                            return;
                        }
                        left -= k as u64;
                    }
                }
            }
        }
    }
    let _ = stream.flush();
}
