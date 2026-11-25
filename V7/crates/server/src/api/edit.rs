//! M3: edit operations (logged, undo/redo), one writer per workspace, uploaded content, save as a new
//! validated version, standalone validation and version deletion. State is in memory (D-039).
use super::*;
use rpf_core::rebuild::{replay, BlobSource, Op};
use rpf_core::validate::{has_errors, validate, Finding};
use rpf_core::{Code, Fail, Severity};
use std::io::{self, BufReader, BufWriter};

pub const MAX_OPS: usize = 1000;
pub const MAX_VERSIONS: usize = 10;
const LEASE: Duration = Duration::from_secs(30);
const DRAIN_MAX: u64 = 8 << 20;

pub struct LoggedOp {
    pub op: Op,
}
pub struct Edit {
    pub base: String,
    pub ops: Vec<LoggedOp>,
    pub cursor: usize,
    pub saving: bool,
    pub rev: u64,
}
pub struct Blob {
    pub ws: String,
    pub owner: String,
    pub path: PathBuf,
    pub size: u64,
}

struct FileBlobs(HashMap<u64, PathBuf>);
impl BlobSource for FileBlobs {
    fn open(&self, id: u64) -> io::Result<Box<dyn Read>> {
        let p = self.0.get(&id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such content"))?;
        Ok(Box::new(BufReader::new(File::open(p)?)))
    }
}

fn sev(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}
fn finding_json(f: &Finding) -> String {
    format!("{{\"code\":{},\"severity\":{},\"message\":{},\"next_step\":{},\"detail\":{}}}", q(f.code.as_str()), q(sev(f.severity)), q(f.code.message()), q(f.code.next_step()), q(&f.detail))
}
fn report_json(findings: &[Finding], extra: &str) -> String {
    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warnings = findings.iter().filter(|f| f.severity == Severity::Warning).count();
    format!("{{\"ok\":{},\"errors\":{},\"warnings\":{},\"findings\":[{}]{}}}", errors == 0, errors, warnings, findings.iter().map(finding_json).collect::<Vec<_>>().join(","), extra)
}
fn fail_finding(f: &Fail) -> Finding {
    Finding { code: f.code, severity: f.code.severity(), detail: f.detail.clone() }
}
fn fail_err(f: Fail) -> ApiErr {
    let mut e = err(422, f.code.as_str(), f.code.message(), f.code.next_step());
    e.details = Some(format!("{{\"check_code\":{},\"detail\":{}}}", q(f.code.as_str()), q(&f.detail)));
    e
}
fn op_json(seq: usize, op: &Op, active: bool) -> String {
    let body = match op {
        Op::Rename { path, new_name } => format!("\"type\":\"rename\",\"path\":{},\"new_name\":{}", q(path), q(new_name)),
        Op::Delete { path } => format!("\"type\":\"delete\",\"path\":{}", q(path)),
        Op::Mkdir { parent, name } => format!("\"type\":\"mkdir\",\"parent\":{},\"name\":{}", q(parent), q(name)),
        Op::AddFile { parent, name, len, .. } => format!("\"type\":\"add_file\",\"parent\":{},\"name\":{},\"size\":{}", q(parent), q(name), len),
        Op::Replace { path, len, .. } | Op::ReplaceResource { path, len, .. } => format!("\"type\":\"replace\",\"path\":{},\"size\":{}", q(path), len),
    };
    format!("{{\"seq\":{},{},\"active\":{}}}", seq, body, active)
}
fn tab_id(req: &Req) -> Result<String, ApiErr> {
    let t = req.headers.get("x-tab-id").cloned().unwrap_or_default();
    if t.len() < 8 || t.len() > 64 || !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(bad("invalid_tab", "The request is missing a valid tab identifier.", "Reload the page and try again."));
    }
    Ok(t)
}

impl State {
    fn ws_owned<'a>(g: &'a Inner, sid: &str, wsid: &str) -> Result<&'a Ws, ApiErr> {
        g.ws.get(wsid).filter(|w| w.owner == sid).ok_or_else(not_found)
    }

    /// Claims or renews the single writer lease for a workspace (D-020).
    fn acquire(g: &mut Inner, wsid: &str, tab: &str) -> Result<(), ApiErr> {
        if let Some((t, at)) = g.writers.get(wsid) {
            if t != tab && at.elapsed() < LEASE {
                return Err(err(409, "writer_busy", "This workspace is open for editing in another tab, so this tab is read-only.", "Close the other tab, or wait about 30 seconds after it stops, then try again."));
            }
        }
        g.writers.insert(wsid.to_string(), (tab.to_string(), Instant::now()));
        Ok(())
    }

    fn state_json(g: &Inner, wsid: &str, tab: &str) -> String {
        let writer = match g.writers.get(wsid) {
            Some((t, at)) if at.elapsed() < LEASE => if t == tab { "you" } else { "other" },
            _ => "none",
        };
        let blob_bytes: u64 = g.blobs.values().filter(|b| b.ws == wsid).map(|b| b.size).sum();
        let (base, cursor, ops, saving) = match g.edits.get(wsid) {
            Some(e) => (q(&e.base), e.cursor, e.ops.iter().enumerate().map(|(i, o)| op_json(i, &o.op, i < e.cursor)).collect::<Vec<_>>(), e.saving),
            None => ("null".to_string(), 0, vec![], false),
        };
        let total = ops.len();
        format!("{{\"workspace_id\":{},\"base_version\":{},\"cursor\":{},\"can_undo\":{},\"can_redo\":{},\"ops\":[{}],\"writer\":{},\"saving\":{},\"max_ops\":{},\"max_versions\":{},\"pending_content_bytes\":{}}}",
            q(wsid), base, cursor, cursor > 0, cursor < total, ops.join(","), q(writer), saving, MAX_OPS, MAX_VERSIONS, blob_bytes)
    }

    fn drop_blobs(g: &mut Inner, wsid: &str) {
        let ids: Vec<u64> = g.blobs.iter().filter(|(_, b)| b.ws == wsid).map(|(k, _)| *k).collect();
        for id in ids {
            if let Some(b) = g.blobs.remove(&id) {
                let _ = fs::remove_file(b.path);
            }
        }
    }

    pub(super) fn route_edit(self: &Arc<Self>, req: &mut Req, sid: &str, seg: &[&str]) -> Result<Resp, ApiErr> {
        let m = req.method.clone();
        match (m.as_str(), seg) {
            ("GET", ["workspaces", id, "edit"]) => {
                let tab = req.headers.get("x-tab-id").cloned().unwrap_or_default();
                let g = self.inner.lock().unwrap();
                Self::ws_owned(&g, sid, id)?;
                Ok(Resp::json(200, Self::state_json(&g, id, &tab)))
            }
            ("GET", ["workspaces", id, "edit", "tree"]) => self.edit_tree(req, sid, id),
            ("POST", ["workspaces", id, "writer"]) => {
                let tab = tab_id(req)?;
                let _ = req.body(MAX_JSON);
                let mut g = self.inner.lock().unwrap();
                Self::ws_owned(&g, sid, id)?;
                Self::acquire(&mut g, id, &tab)?;
                Ok(Resp::json(200, Self::state_json(&g, id, &tab)))
            }
            ("DELETE", ["workspaces", id, "writer"]) => {
                let tab = tab_id(req)?;
                let mut g = self.inner.lock().unwrap();
                Self::ws_owned(&g, sid, id)?;
                if g.writers.get(*id).map_or(false, |(t, _)| *t == tab) {
                    g.writers.remove(*id);
                }
                Ok(Resp::empty(204))
            }
            ("PUT", ["workspaces", id, "blobs"]) => self.blob_put(req, sid, id),
            ("POST", ["workspaces", id, "blobs", "from-version"]) => self.blob_from_version(req, sid, id),
            ("POST", ["workspaces", id, "edit", "ops"]) => self.op_add(req, sid, id),
            ("POST", ["workspaces", id, "edit", which @ ("undo" | "redo")]) => {
                let tab = tab_id(req)?;
                let _ = req.body(MAX_JSON);
                let mut g = self.inner.lock().unwrap();
                Self::ws_owned(&g, sid, id)?;
                Self::acquire(&mut g, id, &tab)?;
                let e = g.edits.get_mut(*id).ok_or_else(|| err(409, "nothing_to_undo", "There is nothing to undo or redo.", ""))?;
                if e.saving {
                    return Err(err(409, "save_in_progress", "A save is running.", "Wait for it to finish."));
                }
                if *which == "undo" {
                    if e.cursor == 0 {
                        return Err(err(409, "nothing_to_undo", "There is nothing to undo.", ""));
                    }
                    e.cursor -= 1;
                } else {
                    if e.cursor >= e.ops.len() {
                        return Err(err(409, "nothing_to_redo", "There is nothing to redo.", ""));
                    }
                    e.cursor += 1;
                }
                e.rev += 1;
                Ok(Resp::json(200, Self::state_json(&g, id, &tab)))
            }
            ("POST", ["workspaces", id, "edit", "discard"]) => {
                let tab = tab_id(req)?;
                let _ = req.body(MAX_JSON);
                let mut g = self.inner.lock().unwrap();
                Self::ws_owned(&g, sid, id)?;
                Self::acquire(&mut g, id, &tab)?;
                if g.edits.get(*id).map_or(false, |e| e.saving) {
                    return Err(err(409, "save_in_progress", "A save is running.", "Wait for it to finish."));
                }
                g.edits.remove(*id);
                Self::drop_blobs(&mut g, id);
                Ok(Resp::json(200, Self::state_json(&g, id, &tab)))
            }
            ("POST", ["workspaces", id, "edit", "validate"]) => self.edit_validate(req, sid, id),
            ("POST", ["workspaces", id, "edit", "save"]) => self.edit_save(req, sid, id),
            ("POST", ["versions", vid, "validate"]) => {
                let _ = req.body(MAX_JSON);
                let (idx, path) = match self.index_for(sid, vid) {
                    Ok(x) => x,
                    Err(e) if e.status == 409 && e.code == "version_failed" => return Err(e),
                    Err(e) => return Err(e),
                };
                let _ = idx;
                let mut f = File::open(&path).map_err(|_| not_found())?;
                let (_, findings) = validate(&mut BufReader::new(&mut f), &self.cfg.limits);
                Ok(Resp::json(200, report_json(&findings, "")))
            }
            ("DELETE", ["versions", vid]) => {
                let mut g = self.inner.lock().unwrap();
                let v = g.vers.get(*vid).filter(|v| v.owner == sid).ok_or_else(not_found)?;
                let (wsid, path) = (v.ws.clone(), v.path.clone());
                if g.edits.get(&wsid).map_or(false, |e| e.base == *vid && (!e.ops.is_empty() || e.saving)) {
                    return Err(err(409, "version_in_use", "This version has unsaved changes or a save in progress.", "Save or discard the changes first."));
                }
                let count = g.ws.get(&wsid).map_or(0, |w| w.versions.len());
                if count <= 1 {
                    return Err(err(409, "last_version", "The last version of a workspace cannot be deleted.", "Delete the workspace instead."));
                }
                g.vers.remove(*vid);
                g.cache.retain(|(id, _)| id != vid);
                if let Some(w) = g.ws.get_mut(&wsid) {
                    w.versions.retain(|x| x != vid);
                }
                if g.edits.get(&wsid).map_or(false, |e| e.base == *vid) {
                    g.edits.remove(&wsid);
                }
                let _ = fs::remove_file(path);
                Ok(Resp::empty(204))
            }
            _ => Err(not_found()),
        }
    }

    fn blob_put(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let r = self.blob_put_inner(req, sid, wsid);
        if r.is_err() {
            req.drain_body(DRAIN_MAX);
        }
        r
    }

    fn blob_put_inner(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let tab = tab_id(req)?;
        let size = req.content_length;
        if size > self.cfg.max_blob {
            let mut e = err(413, "limit_exceeded", format!("The file is {} bytes. The limit for one added file is {} bytes.", size, self.cfg.max_blob), "Choose a smaller file.");
            e.details = Some(format!("{{\"limit\":\"max_blob_bytes\",\"value\":{},\"max\":{}}}", size, self.cfg.max_blob));
            return Err(e);
        }
        let (id, path) = {
            let mut g = self.inner.lock().unwrap();
            Self::ws_owned(&g, sid, wsid)?;
            Self::acquire(&mut g, wsid, &tab)?;
            let used = Self::usage(&g, sid);
            self.server_full(&g, size)?;
            if used + size > self.cfg.quota {
                let mut e = err(413, "quota_exceeded", format!("This file would use {} bytes of your {} byte storage allowance ({} already used).", size, self.cfg.quota, used), "Discard pending changes or delete a version to free space.");
                e.details = Some(format!("{{\"limit\":\"session_quota_bytes\",\"value\":{},\"max\":{}}}", used + size, self.cfg.quota));
                return Err(e);
            }
            let raw = u64::from_str_radix(&rand_hex(8), 16).unwrap_or(1);
            let id = (raw & ((1u64 << 52) - 1)).max(1);
            let path = self.cfg.data_dir.join(wsid).join(format!("blob-{}.bin", id));
            // reserve the bytes so concurrent uploads cannot exceed the quota together
            g.blobs.insert(id, Blob { ws: wsid.into(), owner: sid.into(), path: path.clone(), size });
            (id, path)
        };
        let res = (|| -> Result<(), ()> {
            let f = File::create(&path).map_err(|_| ())?;
            let mut w = BufWriter::new(f);
            req.stream_body_to(&mut w).map_err(|_| ())?;
            std::io::Write::flush(&mut w).map_err(|_| ())
        })();
        if res.is_err() {
            let mut g = self.inner.lock().unwrap();
            g.blobs.remove(&id);
            let _ = fs::remove_file(&path);
            return Err(bad("body_incomplete", "The file did not arrive completely.", "Add the file again."));
        }
        Ok(Resp::json(201, format!("{{\"blob_id\":{},\"size\":{}}}", id, size)))
    }


    /// Copies a ready version of this workspace into pending content, so it can replace an entry in its parent archive.
    fn blob_from_version(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let tab = tab_id(req)?;
        let body = req.body(MAX_JSON)?;
        let o = parse_flat(&String::from_utf8_lossy(&body)).ok_or_else(|| bad("invalid_json", "The request body is not valid.", "Send a JSON object."))?;
        let version = get_str(&o, "version").ok_or_else(|| bad("invalid_request", "The version is missing.", "Reload the page and try again."))?;
        let (src, size, parent) = {
            let mut g = self.inner.lock().unwrap();
            Self::ws_owned(&g, sid, wsid)?;
            Self::acquire(&mut g, wsid, &tab)?;
            let v = g.vers.get(&version).filter(|v| v.owner == sid && v.ws == wsid && matches!(v.state, VState::Ready)).ok_or_else(not_found)?;
            let parent = v.parent.clone().ok_or_else(|| err(422, "no_parent_archive", "This version was not opened from a nested archive.", "Open a nested archive first."))?;
            if v.size > self.cfg.max_blob {
                return Err(err(413, "limit_exceeded", format!("The archive is {} bytes. The limit for one added file is {} bytes.", v.size, self.cfg.max_blob), "Use a smaller archive."));
            }
            if Self::usage(&g, sid) + v.size > self.cfg.quota {
                return Err(err(413, "quota_exceeded", "Writing this archive back would exceed your storage allowance.", "Delete an older version, then try again."));
            }
            self.server_full(&g, v.size)?;
            (v.path.clone(), v.size, parent)
        };
        let raw = u64::from_str_radix(&rand_hex(8), 16).unwrap_or(1);
        let id = (raw & ((1u64 << 52) - 1)).max(1);
        let path = self.cfg.data_dir.join(wsid).join(format!("blob-{}.bin", id));
        if fs::copy(&src, &path).is_err() {
            let _ = fs::remove_file(&path);
            return Err(err(503, "storage_error", "Storage is unavailable.", "Try again later."));
        }
        let mut g = self.inner.lock().unwrap();
        g.blobs.insert(id, Blob { ws: wsid.into(), owner: sid.into(), path, size });
        Ok(Resp::json(201, format!("{{\"blob_id\":{},\"size\":{},\"parent_version\":{},\"parent_path\":{}}}", id, size, q(&parent.0), q(&parent.1))))
    }

    fn op_add(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let tab = tab_id(req)?;
        let body = req.body(MAX_JSON)?;
        let o = parse_flat(&String::from_utf8_lossy(&body)).ok_or_else(|| bad("invalid_json", "The request body is not valid.", "Send a JSON object."))?;
        let version = get_str(&o, "version").ok_or_else(|| bad("invalid_request", "The version is missing.", "Reload the page and try again."))?;
        let kind = get_str(&o, "op").unwrap_or_default();
        let s = |k: &str| get_str(&o, k).ok_or_else(|| bad("invalid_request", &format!("The field \"{}\" is missing.", k), "Check the request and try again."));
        // Phase 1 (under the lock): permissions, base version, the new operation, a snapshot of the log.
        let (snapshot, rev, new_op) = {
            let mut g = self.inner.lock().unwrap();
            Self::ws_owned(&g, sid, wsid)?;
            Self::acquire(&mut g, wsid, &tab)?;
            let ver_ok = g.vers.get(&version).map_or(false, |v| v.owner == sid && v.ws == wsid && matches!(v.state, VState::Ready));
            if !ver_ok {
                return Err(not_found());
            }
            let blob_len = |g: &Inner, key: &str| -> Result<(u64, u64), ApiErr> {
                let id = get_u64(&o, key).ok_or_else(|| bad("invalid_request", "The uploaded content reference is missing.", "Add the file again."))?;
                let b = g.blobs.get(&id).filter(|b| b.ws == wsid && b.owner == sid).ok_or_else(|| bad("content_missing", "The uploaded content is no longer available.", "Add the file again."))?;
                Ok((id, b.size))
            };
            let new_op = match kind.as_str() {
                "rename" => Op::Rename { path: s("path")?, new_name: s("new_name")? },
                "delete" => Op::Delete { path: s("path")? },
                "mkdir" => Op::Mkdir { parent: s("parent")?, name: s("name")? },
                "add_file" => {
                    let (blob, len) = blob_len(&g, "blob_id")?;
                    Op::AddFile { parent: s("parent")?, name: s("name")?, blob, len }
                }
                "replace" => {
                    let (blob, len) = blob_len(&g, "blob_id")?;
                    // New content that starts with an RSC7 header replaces a resource entry (page flags come from that header).
                    let head = g.blobs.get(&blob).and_then(|b| {
                        let mut h = [0u8; 16];
                        File::open(&b.path).ok()?.read_exact(&mut h).ok()?;
                        Some(h)
                    });
                    match head.and_then(|h| rpf_core::rsc::baslik(&h).map(|(sys, gfx)| (sys, gfx))) {
                        Some((sys, gfx)) if len >= 16 => Op::ReplaceResource { path: s("path")?, blob, len, sys, gfx },
                        _ => Op::Replace { path: s("path")?, blob, len },
                    }
                }
                _ => return Err(bad("invalid_request", "The operation type is not supported.", "Use rename, delete, mkdir, add_file or replace.")),
            };
            match g.edits.get(wsid) {
                Some(e) if e.base != version => {
                    if e.ops.is_empty() && !e.saving {
                        g.edits.remove(wsid);
                    } else {
                        return Err(err(409, "edit_base_mismatch", "There are unsaved changes on another version.", "Save or discard the pending changes first."));
                    }
                }
                _ => {}
            }
            let e = g.edits.entry(wsid.to_string()).or_insert_with(|| Edit { base: version.clone(), ops: vec![], cursor: 0, saving: false, rev: 0 });
            if e.saving {
                return Err(err(409, "save_in_progress", "A save is running.", "Wait for it to finish."));
            }
            if e.cursor >= MAX_OPS {
                return Err(err(409, "limit_exceeded", format!("A workspace can hold at most {} pending changes.", MAX_OPS), "Save the changes, then continue."));
            }
            (e.ops[..e.cursor].iter().map(|l| l.op.clone()).collect::<Vec<_>>(), e.rev, new_op)
        };
        // Phase 2 (no lock): replay the log plus the new operation on the base index; this validates it.
        let (idx, _) = self.index_for(sid, &version)?;
        let mut all = snapshot;
        all.push(new_op.clone());
        replay(&idx, &all, &self.cfg.limits).map_err(|(_, f)| fail_err(f))?;
        // Phase 3: append, dropping any redo tail.
        let mut g = self.inner.lock().unwrap();
        let e = g.edits.get_mut(wsid).ok_or_else(|| err(409, "conflict", "The pending changes were modified elsewhere.", "Reload and try again."))?;
        if e.rev != rev || e.saving {
            return Err(err(409, "conflict", "The pending changes were modified elsewhere.", "Reload and try again."));
        }
        let cur = e.cursor;
        e.ops.truncate(cur);
        e.ops.push(LoggedOp { op: new_op });
        e.cursor += 1;
        e.rev += 1;
        Ok(Resp::json(200, Self::state_json(&g, wsid, &tab)))
    }

    /// Dry run: replays the active log and plans the new archive without writing anything.
    /// Live preview: children of one folder after the pending edits of this workspace (read-only, no writer lease needed).
    fn edit_tree(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let version = req.query.get("version").cloned().unwrap_or_default();
        let path = req.query.get("path").cloned().unwrap_or_else(|| "/".to_string());
        let limit = req.query.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(200).clamp(1, 500);
        let cursor = req.query.get("cursor").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let (base, ops) = {
            let g = self.inner.lock().unwrap();
            Self::ws_owned(&g, sid, wsid)?;
            match g.edits.get(wsid) {
                Some(e) if e.base == version => (e.base.clone(), e.ops[..e.cursor].iter().map(|l| l.op.clone()).collect::<Vec<_>>()),
                _ => (version.clone(), Vec::new()),
            }
        };
        let (idx, _) = self.index_for(sid, &base)?;
        let tree = replay(&idx, &ops, &self.cfg.limits).map_err(|(_, f)| fail_err(f))?;
        let all = tree.list_folder(&path).map_err(fail_err)?;
        let total = all.len();
        let items: Vec<String> = all.iter().skip(cursor).take(limit)
            .map(|(n, d, sz, new)| format!("{{\"name\":{},\"kind\":\"{}\",\"size\":{},\"new\":{}}}", q(n), if *d { "directory" } else { "file" }, sz, new))
            .collect();
        Ok(Resp::json(200, format!("{{\"path\":{},\"total\":{},\"entries\":[{}]}}", q(&path), total, items.join(","))))
    }

    fn edit_validate(&self, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let tab = tab_id(req)?;
        let _ = req.body(MAX_JSON);
        let (base, ops) = {
            let mut g = self.inner.lock().unwrap();
            Self::ws_owned(&g, sid, wsid)?;
            Self::acquire(&mut g, wsid, &tab)?;
            let e = g.edits.get(wsid).ok_or_else(|| err(409, "nothing_to_save", "There are no pending changes.", "Make a change first."))?;
            (e.base.clone(), e.ops[..e.cursor].iter().map(|l| l.op.clone()).collect::<Vec<_>>())
        };
        let (idx, _) = self.index_for(sid, &base)?;
        let result = replay(&idx, &ops, &self.cfg.limits).map_err(|(_, f)| f).and_then(|t| t.plan(&self.cfg.limits));
        let body = match result {
            Ok(l) => report_json(&[], &format!(",\"planned\":{{\"entries\":{},\"file_bytes\":{},\"copied_entries\":{},\"new_entries\":{}}}", l.entry_count, l.file_len, l.copied_entries, l.new_entries)),
            Err(f) => report_json(&[fail_finding(&f)], ""),
        };
        Ok(Resp::json(200, body))
    }

    fn edit_save(self: &Arc<Self>, req: &mut Req, sid: &str, wsid: &str) -> Result<Resp, ApiErr> {
        let tab = tab_id(req)?;
        let _ = req.body(MAX_JSON);
        let (base, ops, blobs, jid) = {
            let mut g = self.inner.lock().unwrap();
            let w = Self::ws_owned(&g, sid, wsid)?;
            let nver = w.versions.len();
            Self::acquire(&mut g, wsid, &tab)?;
            if nver >= MAX_VERSIONS {
                return Err(err(409, "version_cap", format!("A workspace can keep at most {} versions.", MAX_VERSIONS), "Delete an older version, then save again."));
            }
            let blobs: HashMap<u64, PathBuf> = g.blobs.iter().filter(|(_, b)| b.ws == wsid).map(|(k, b)| (*k, b.path.clone())).collect();
            let e = g.edits.get_mut(wsid).filter(|e| e.cursor > 0).ok_or_else(|| err(409, "nothing_to_save", "There are no pending changes to save.", "Make a change first."))?;
            if e.saving {
                return Err(err(409, "save_in_progress", "A save is already running.", "Wait for it to finish."));
            }
            e.saving = true;
            let ops: Vec<Op> = e.ops[..e.cursor].iter().map(|l| l.op.clone()).collect();
            let base = e.base.clone();
            let jid = rand_hex(16);
            g.jobs.insert(jid.clone(), Job { owner: sid.into(), kind: "save", state: "queued", progress: 0, error: None, version: None, report: None });
            (base, ops, blobs, jid)
        };
        let me = self.clone();
        let (j2, w2, s2) = (jid.clone(), wsid.to_string(), sid.to_string());
        std::thread::spawn(move || me.run_save(j2, s2, w2, base, ops, blobs));
        Ok(Resp::json(202, format!("{{\"job_id\":{}}}", q(&jid))))
    }

    fn job_progress(&self, jid: &str, p: u32) {
        if let Some(j) = self.inner.lock().unwrap().jobs.get_mut(jid) {
            j.state = "running";
            j.progress = p;
        }
    }

    /// Ends a failed save: nothing is published, the temp file is removed, the previous version is untouched.
    fn save_failed(&self, jid: &str, wsid: &str, tmp: Option<&PathBuf>, findings: Vec<Finding>, fallback: (String, String, String)) {
        if let Some(t) = tmp {
            let _ = fs::remove_file(t);
        }
        let mut g = self.inner.lock().unwrap();
        if let Some(e) = g.edits.get_mut(wsid) {
            e.saving = false;
            e.rev += 1;
        }
        let triple = match findings.iter().find(|f| f.severity == Severity::Error) {
            Some(f) => (f.code.as_str().to_string(), f.code.message().to_string(), f.code.next_step().to_string()),
            None => fallback,
        };
        if let Some(j) = g.jobs.get_mut(jid) {
            j.state = "failed";
            j.progress = 100;
            j.error = Some(triple);
            j.report = Some(report_json(&findings, ""));
        }
    }

    fn run_save(self: &Arc<Self>, jid: String, sid: String, wsid: String, base: String, ops: Vec<Op>, blobs: HashMap<u64, PathBuf>) {
        self.job_progress(&jid, 5);
        let internal = ("IO_ERROR".to_string(), Code::Io.message().to_string(), Code::Io.next_step().to_string());
        let Ok((idx, src_path)) = self.index_for(&sid, &base) else {
            return self.save_failed(&jid, &wsid, None, vec![], internal);
        };
        let layout = match replay(&idx, &ops, &self.cfg.limits).map_err(|(_, f)| f).and_then(|t| t.plan(&self.cfg.limits)) {
            Ok(l) => l,
            Err(f) => return self.save_failed(&jid, &wsid, None, vec![fail_finding(&f)], internal),
        };
        // Storage quota is enforced before the rebuild starts. Pending content is freed after a save, so it is not double counted.
        {
            let g = self.inner.lock().unwrap();
            let pending: u64 = g.blobs.values().filter(|b| b.ws == wsid).map(|b| b.size).sum();
            let used = Self::usage(&g, &sid).saturating_sub(pending);
            if let Err(e) = self.server_full(&g, layout.file_len.saturating_sub(pending)) {
                drop(g);
                return self.save_failed(&jid, &wsid, None, vec![], (e.code.to_string(), e.message.clone(), e.next.clone().unwrap_or_default()));
            }
            if used + layout.file_len > self.cfg.quota {
                drop(g);
                let msg = format!("The saved archive would be {} bytes, which does not fit in your {} byte storage allowance ({} already used).", layout.file_len, self.cfg.quota, used);
                return self.save_failed(&jid, &wsid, None, vec![], ("quota_exceeded".into(), msg, "Delete an older version or remove files, then save again.".into()));
            }
        }
        self.job_progress(&jid, 20);
        let tmp = self.cfg.data_dir.join(&wsid).join(format!("save-{}.tmp", jid));
        let written = (|| -> Result<(), Fail> {
            let io = |e: io::Error| Fail::new(Code::Io, e.to_string());
            let mut src = BufReader::new(File::open(&src_path).map_err(io)?);
            let mut out = BufWriter::with_capacity(1 << 20, File::create(&tmp).map_err(io)?);
            layout.write(&mut src, &FileBlobs(blobs), &mut out)?;
            std::io::Write::flush(&mut out).map_err(io)
        })();
        if let Err(f) = written {
            return self.save_failed(&jid, &wsid, Some(&tmp), vec![fail_finding(&f)], internal);
        }
        self.job_progress(&jid, 70);
        // Validate before publish (D-007, D-021): the rebuilt file must re-parse cleanly and match the plan.
        let checked = File::open(&tmp).map(|f| validate(&mut BufReader::new(f), &self.cfg.limits));
        let (new_idx, mut findings) = match checked {
            Ok((Some(i), f)) => (Some(i), f),
            Ok((None, f)) => (None, f),
            Err(_) => return self.save_failed(&jid, &wsid, Some(&tmp), vec![], internal),
        };
        if let Some(i) = &new_idx {
            if i.len() as u32 != layout.entry_count || i.file_len() != layout.file_len {
                findings.push(Finding { code: Code::RebuildVerifyFailed, severity: Severity::Error, detail: "entry count or length differs from the plan".into() });
            }
        }
        let Some(new_idx) = new_idx.filter(|_| !has_errors(&findings)) else {
            return self.save_failed(&jid, &wsid, Some(&tmp), findings, internal);
        };
        self.job_progress(&jid, 90);
        // Publish.
        let vid = rand_hex(16);
        let vpath = self.cfg.data_dir.join(&wsid).join(format!("v-{}.rpf", vid));
        let mut g = self.inner.lock().unwrap();
        let alive = g.ws.contains_key(&wsid) && g.edits.get(&wsid).map_or(false, |e| e.saving);
        if !alive || fs::rename(&tmp, &vpath).is_err() {
            drop(g);
            return self.save_failed(&jid, &wsid, Some(&tmp), vec![], internal);
        }
        let parent_link = g.vers.get(&base).and_then(|v| v.parent.clone());
        let filename = g.vers.get(&base).map(|v| v.filename.clone()).unwrap_or_else(|| "archive.rpf".into());
        let n = new_idx.len() as u64;
        g.vers.insert(vid.clone(), Ver { id: vid.clone(), ws: wsid.clone(), owner: sid.clone(), path: vpath, size: layout.file_len, filename, state: VState::Ready, entries: n, origin: "save", created: now_unix(), parent: parent_link });
        if let Some(w) = g.ws.get_mut(&wsid) {
            w.versions.push(vid.clone());
        }
        g.cache.push((vid.clone(), Arc::new(new_idx)));
        while g.cache.len() > CACHE_CAP {
            g.cache.remove(0);
        }
        g.edits.remove(&wsid);
        Self::drop_blobs(&mut g, &wsid);
        let extra = format!(",\"saved\":{{\"entries\":{},\"file_bytes\":{},\"copied_entries\":{},\"new_entries\":{}}}", layout.entry_count, layout.file_len, layout.copied_entries, layout.new_entries);
        if let Some(j) = g.jobs.get_mut(&jid) {
            j.state = "succeeded";
            j.progress = 100;
            j.version = Some(vid);
            j.report = Some(report_json(&findings, &extra));
        }
    }
}
