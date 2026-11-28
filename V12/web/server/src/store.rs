//! Per-session workspaces on disk (design: `docs/M1_ARCHITECTURE.md` §3). std-only.
//!
//! Layout: `{root}/{wsDir}/files/{fileId}/original` (immutable upload) and `{root}/{wsDir}/tmp/` (staging).
//! `wsDir` and `fileId` are server-generated random hex, never derived from client input; the session token is a
//! separate random value and is never used as a path component. Server paths never leave this module except via
//! `original_path`, which is for the in-process engine only and must not be serialized to clients.
//!
//! Known limits (honest notes): random bytes come from `/dev/urandom` (Linux/Unix only, fine for Railway);
//! token lookup is a `HashMap` lookup, not a constant-time compare — acceptable only because tokens are 256-bit random
//! and format-validated first, but the HTTP layer may add a constant-time check. No `meta.json`, backups, or `current`
//! file yet (later M2/M4 batches).

use crate::config::Config;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    TooManyWorkspaces,
    TooManyFiles,
    UploadTooLarge,
    QuotaExceeded,
    InvalidName,
    NotFound,
    Io,
}

impl StoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooManyWorkspaces => "SERVER_BUSY",
            Self::TooManyFiles => "TOO_MANY_FILES",
            Self::UploadTooLarge => "UPLOAD_TOO_LARGE",
            Self::QuotaExceeded => "WORKSPACE_QUOTA_EXCEEDED",
            Self::InvalidName => "INVALID_FILE_NAME",
            Self::NotFound => "NOT_FOUND",
            Self::Io => "IO_ERROR",
        }
    }
    /// English, path-free client message.
    pub fn message(&self) -> &'static str {
        match self {
            Self::TooManyWorkspaces => "The server has reached its workspace limit. Try again later.",
            Self::TooManyFiles => "This workspace has reached its file limit.",
            Self::UploadTooLarge => "The file exceeds the maximum upload size.",
            Self::QuotaExceeded => "This workspace has reached its storage limit.",
            Self::InvalidName => "The file name is not valid.",
            Self::NotFound => "The requested item was not found.",
            Self::Io => "A storage error occurred.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMeta {
    pub file_id: String,
    pub name: String,
    pub size: u64,
}

/// One saved backup of a file's previous `current` version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupMeta {
    pub id: String,
    pub size: u64,
    /// Seconds since the Unix epoch when the backup was taken.
    pub created_unix: u64,
}

/// Maximum backups kept per file (oldest are removed first), same as the desktop app.
pub const MAX_BACKUPS_PER_FILE: usize = 20;

#[derive(Debug, Default)]
struct FileState {
    /// Size of `files/{id}/current`, `None` while the file was never saved (original is the live version).
    current_size: Option<u64>,
    /// Oldest first.
    backups: Vec<BackupMeta>,
    next_backup: u64,
}

#[derive(Debug)]
struct Workspace {
    dir: String,
    created: Instant,
    last_used: Instant,
    files: Vec<FileMeta>,
    state: HashMap<String, FileState>,
}

impl Workspace {
    /// Everything counted against the workspace quota: originals, saved versions and backups.
    fn bytes(&self) -> u64 {
        let extra: u64 = self.state.values().map(|s| s.current_size.unwrap_or(0) + s.backups.iter().map(|b| b.size).sum::<u64>()).sum();
        self.files.iter().map(|f| f.size).sum::<u64>() + extra
    }
}

#[derive(Debug)]
pub struct WorkspaceStore {
    pub cfg: Config,
    inner: Mutex<HashMap<String, Workspace>>,
}

fn random_hex(bytes: usize) -> Result<String, StoreError> {
    let mut buf = vec![0u8; bytes];
    fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)).map_err(|_| StoreError::Io)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn valid_token(t: &str) -> bool {
    t.len() == 64 && t.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Strips any directory part, control characters and surrounding whitespace; caps length at 255 bytes.
pub fn sanitize_file_name(raw: &str) -> Result<String, StoreError> {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base.chars().filter(|c| !c.is_control()).collect();
    let mut name = cleaned.trim().to_string();
    while name.len() > 255 {
        name.pop();
    }
    if name.is_empty() || name == "." || name == ".." {
        return Err(StoreError::InvalidName);
    }
    Ok(name)
}

impl WorkspaceStore {
    pub fn new(cfg: Config) -> Result<Self, StoreError> {
        fs::create_dir_all(&cfg.workspace_root).map_err(|_| StoreError::Io)?;
        Ok(Self { cfg, inner: Mutex::new(HashMap::new()) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Workspace>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn ws_path(&self, dir: &str) -> PathBuf {
        self.cfg.workspace_root.join(dir)
    }

    /// Returns a usable token: the given one if it names a live workspace, otherwise a brand-new empty workspace.
    /// An unknown or malformed token is indistinguishable from "no token" (never reveals other workspaces).
    pub fn session(&self, token: Option<&str>) -> Result<String, StoreError> {
        let now = Instant::now();
        let mut map = self.lock();
        if let Some(t) = token.filter(|t| valid_token(t)) {
            if let Some(ws) = map.get_mut(t) {
                ws.last_used = now;
                return Ok(t.to_string());
            }
        }
        if map.len() >= self.cfg.max_workspaces {
            return Err(StoreError::TooManyWorkspaces);
        }
        let new_token = random_hex(32)?;
        let dir = random_hex(16)?;
        let path = self.ws_path(&dir);
        fs::create_dir_all(path.join("files")).and_then(|_| fs::create_dir_all(path.join("tmp"))).map_err(|_| StoreError::Io)?;
        map.insert(new_token.clone(), Workspace { dir, created: now, last_used: now, files: Vec::new(), state: HashMap::new() });
        Ok(new_token)
    }

    fn touch_dir(&self, token: &str) -> Result<String, StoreError> {
        let mut map = self.lock();
        let ws = map.get_mut(token).filter(|_| valid_token(token)).ok_or(StoreError::NotFound)?;
        ws.last_used = Instant::now();
        Ok(ws.dir.clone())
    }

    /// Streams an upload to staging with the size cap enforced while reading, fsyncs, then renames into place.
    pub fn add_file(&self, token: &str, display_name: &str, data: &mut impl Read) -> Result<FileMeta, StoreError> {
        let name = sanitize_file_name(display_name)?;
        let dir = self.touch_dir(token)?;
        {
            let map = self.lock();
            let ws = map.get(token).ok_or(StoreError::NotFound)?;
            if ws.files.len() >= self.cfg.max_files_per_workspace {
                return Err(StoreError::TooManyFiles);
            }
        }
        let ws_path = self.ws_path(&dir);
        let tmp = ws_path.join("tmp").join(random_hex(16)?);
        let file_id = random_hex(16)?;
        let final_dir = ws_path.join("files").join(&file_id);
        let result = (|| -> Result<u64, StoreError> {
            let mut out = fs::File::create(&tmp).map_err(|_| StoreError::Io)?;
            let mut buf = vec![0u8; 64 * 1024];
            let mut total: u64 = 0;
            loop {
                let n = data.read(&mut buf).map_err(|_| StoreError::Io)?;
                if n == 0 {
                    break;
                }
                total += n as u64;
                if total > self.cfg.max_upload_bytes {
                    return Err(StoreError::UploadTooLarge);
                }
                out.write_all(&buf[..n]).map_err(|_| StoreError::Io)?;
            }
            out.sync_all().map_err(|_| StoreError::Io)?;
            Ok(total)
        })();
        let size = match result {
            Ok(s) => s,
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                return Err(e);
            }
        };
        // Re-check quota/count under the lock, then commit (IO happened outside the lock; races are re-validated here).
        let mut map = self.lock();
        let commit = (|| -> Result<FileMeta, StoreError> {
            let ws = map.get_mut(token).ok_or(StoreError::NotFound)?;
            if ws.files.len() >= self.cfg.max_files_per_workspace {
                return Err(StoreError::TooManyFiles);
            }
            if ws.bytes() + size > self.cfg.max_workspace_bytes {
                return Err(StoreError::QuotaExceeded);
            }
            fs::create_dir_all(&final_dir).map_err(|_| StoreError::Io)?;
            fs::rename(&tmp, final_dir.join("original")).map_err(|_| StoreError::Io)?;
            let meta = FileMeta { file_id: file_id.clone(), name, size };
            ws.files.push(meta.clone());
            Ok(meta)
        })();
        if commit.is_err() {
            let _ = fs::remove_file(&tmp);
            let _ = fs::remove_dir_all(&final_dir);
        }
        commit
    }

    pub fn list_files(&self, token: &str) -> Result<Vec<FileMeta>, StoreError> {
        self.touch_dir(token)?;
        Ok(self.lock().get(token).ok_or(StoreError::NotFound)?.files.clone())
    }

    /// Internal-only absolute path to the immutable upload, for the engine to read. A fileId that is not in THIS
    /// workspace yields `NotFound`, exactly like a nonexistent id.
    pub fn original_path(&self, token: &str, file_id: &str) -> Result<PathBuf, StoreError> {
        let dir = self.touch_dir(token)?;
        let map = self.lock();
        let ws = map.get(token).ok_or(StoreError::NotFound)?;
        if !ws.files.iter().any(|f| f.file_id == file_id) {
            return Err(StoreError::NotFound);
        }
        Ok(self.ws_path(&dir).join("files").join(file_id).join("original"))
    }

    fn file_dir(&self, token: &str, file_id: &str) -> Result<PathBuf, StoreError> {
        let dir = self.touch_dir(token)?;
        let map = self.lock();
        let ws = map.get(token).ok_or(StoreError::NotFound)?;
        if !ws.files.iter().any(|f| f.file_id == file_id) {
            return Err(StoreError::NotFound);
        }
        Ok(self.ws_path(&dir).join("files").join(file_id))
    }

    pub fn file_meta(&self, token: &str, file_id: &str) -> Result<FileMeta, StoreError> {
        self.touch_dir(token)?;
        let map = self.lock();
        map.get(token).and_then(|w| w.files.iter().find(|f| f.file_id == file_id).cloned()).ok_or(StoreError::NotFound)
    }

    /// Internal-only path of the live version: the last saved `current`, otherwise the immutable original.
    pub fn current_path(&self, token: &str, file_id: &str) -> Result<PathBuf, StoreError> {
        let d = self.file_dir(token, file_id)?;
        let has = self.lock().get(token).and_then(|w| w.state.get(file_id)).is_some_and(|s| s.current_size.is_some());
        Ok(d.join(if has { "current" } else { "original" }))
    }

    /// Saves a new live version. The previous `current` (if any) is kept as a backup first; the original upload is
    /// never modified, so it needs no backup. Old backups are dropped to stay within the file's backup cap and the
    /// workspace quota. The new version is written to staging, fsynced, then renamed over `current` (atomic).
    /// Returns `(old_size, new_size, backup_id)` where `old_size` is the size of the version that was replaced.
    pub fn save_version(&self, token: &str, file_id: &str, bytes: &[u8]) -> Result<(u64, u64, Option<String>), StoreError> {
        let fdir = self.file_dir(token, file_id)?;
        let ws_dir = fdir.parent().and_then(|p| p.parent()).ok_or(StoreError::Io)?.to_path_buf();
        let tmp = ws_dir.join("tmp").join(random_hex(16)?);
        let new_size = bytes.len() as u64;
        let write = (|| -> std::io::Result<()> {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()
        })();
        if write.is_err() {
            let _ = fs::remove_file(&tmp);
            return Err(StoreError::Io);
        }
        let mut map = self.lock();
        let result = (|| -> Result<(u64, u64, Option<String>), StoreError> {
            let ws = map.get_mut(token).ok_or(StoreError::NotFound)?;
            let orig_size = ws.files.iter().find(|f| f.file_id == file_id).map(|f| f.size).ok_or(StoreError::NotFound)?;
            let st = ws.state.entry(file_id.to_string()).or_default();
            let old_current = st.current_size;
            let old_size = old_current.unwrap_or(orig_size);
            // Make room: drop oldest backups until the projected total fits.
            let mut dropped = Vec::new();
            loop {
                let ws_ref = &*ws;
                let projected = ws_ref.bytes() - old_current.unwrap_or(0) + new_size + old_current.unwrap_or(0);
                if projected <= self.cfg.max_workspace_bytes {
                    break;
                }
                let ws_mut = &mut *ws;
                let victim = ws_mut.state.get_mut(file_id).and_then(|s| (!s.backups.is_empty()).then(|| s.backups.remove(0)));
                match victim {
                    Some(b) => dropped.push(b.id),
                    None => {
                        for id in dropped {
                            let _ = fs::remove_file(fdir.join("backups").join(&id));
                        }
                        return Err(StoreError::QuotaExceeded);
                    }
                }
            }
            for id in &dropped {
                let _ = fs::remove_file(fdir.join("backups").join(id));
            }
            let st = ws.state.get_mut(file_id).ok_or(StoreError::NotFound)?;
            let mut backup_id = None;
            if old_current.is_some() {
                fs::create_dir_all(fdir.join("backups")).map_err(|_| StoreError::Io)?;
                st.next_backup += 1;
                let id = format!("{:06}", st.next_backup);
                fs::copy(fdir.join("current"), fdir.join("backups").join(&id)).map_err(|_| StoreError::Io)?;
                let created_unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                st.backups.push(BackupMeta { id: id.clone(), size: old_size, created_unix });
                backup_id = Some(id);
                while st.backups.len() > MAX_BACKUPS_PER_FILE {
                    let old = st.backups.remove(0);
                    let _ = fs::remove_file(fdir.join("backups").join(old.id));
                }
            }
            fs::rename(&tmp, fdir.join("current")).map_err(|_| StoreError::Io)?;
            st.current_size = Some(new_size);
            Ok((old_size, new_size, backup_id))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    /// Backups of a file, newest first.
    pub fn list_backups(&self, token: &str, file_id: &str) -> Result<Vec<BackupMeta>, StoreError> {
        self.file_dir(token, file_id)?;
        let map = self.lock();
        let mut v = map.get(token).and_then(|w| w.state.get(file_id)).map(|s| s.backups.clone()).unwrap_or_default();
        v.reverse();
        Ok(v)
    }

    /// Internal path of a backup. The id must match a known backup of this file (no client-supplied path joining).
    pub fn backup_path(&self, token: &str, file_id: &str, backup_id: &str) -> Result<PathBuf, StoreError> {
        let fdir = self.file_dir(token, file_id)?;
        let map = self.lock();
        let known = map.get(token).and_then(|w| w.state.get(file_id)).is_some_and(|s| s.backups.iter().any(|b| b.id == backup_id));
        if !known {
            return Err(StoreError::NotFound);
        }
        Ok(fdir.join("backups").join(backup_id))
    }

    /// Deletes the workspace and all its data immediately. Unknown token is a no-op.
    pub fn delete_session(&self, token: &str) {
        let removed = self.lock().remove(token);
        if let Some(ws) = removed {
            let _ = fs::remove_dir_all(self.ws_path(&ws.dir));
        }
    }

    /// Removes workspaces idle longer than `idle_ttl` or older than `max_age` (relative to `now`).
    /// Returns the removed session tokens so callers can drop state they keep per session.
    pub fn sweep_at(&self, now: Instant) -> Vec<String> {
        let expired: Vec<(String, Workspace)> = {
            let mut map = self.lock();
            let keys: Vec<String> = map
                .iter()
                .filter(|(_, w)| {
                    now.saturating_duration_since(w.last_used) > self.cfg.idle_ttl
                        || now.saturating_duration_since(w.created) > self.cfg.max_age
                })
                .map(|(k, _)| k.clone())
                .collect();
            keys.into_iter().filter_map(|k| map.remove(&k).map(|w| (k, w))).collect()
        };
        for (_, ws) in &expired {
            let _ = fs::remove_dir_all(self.ws_path(&ws.dir));
        }
        expired.into_iter().map(|(k, _)| k).collect()
    }

    pub fn sweep(&self) -> Vec<String> {
        self.sweep_at(Instant::now())
    }

    /// Readiness: the workspace root exists and a file can be created in it.
    pub fn ready(&self) -> bool {
        let probe = self.cfg.workspace_root.join(".ready-probe");
        fs::write(&probe, b"ok").is_ok() && fs::remove_file(&probe).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    fn store(tweak: impl FnOnce(&mut Config)) -> (WorkspaceStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("zyn-test-{}", random_hex(8).unwrap()));
        let mut cfg = Config { workspace_root: root.clone(), ..Config::default() };
        tweak(&mut cfg);
        (WorkspaceStore::new(cfg).unwrap(), root)
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_file_name("C:\\mods\\a.rpf").unwrap(), "a.rpf");
        assert_eq!(sanitize_file_name("../../etc/passwd").unwrap(), "passwd");
        assert_eq!(sanitize_file_name(" x\u{0}y.rpf ").unwrap(), "xy.rpf");
        for bad in ["", "..", "dir/", "   ", "\n"] {
            assert_eq!(sanitize_file_name(bad), Err(StoreError::InvalidName), "{bad:?}");
        }
        assert!(sanitize_file_name(&"a".repeat(1000)).unwrap().len() <= 255);
    }

    #[test]
    fn upload_list_and_path() {
        let (s, root) = store(|_| {});
        let t = s.session(None).unwrap();
        assert_eq!(t.len(), 64);
        let m = s.add_file(&t, "../x.rpf", &mut Cursor::new(b"hello".to_vec())).unwrap();
        assert_eq!((m.name.as_str(), m.size), ("x.rpf", 5));
        assert_eq!(s.list_files(&t).unwrap(), vec![m.clone()]);
        let p = s.original_path(&t, &m.file_id).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"hello");
        assert!(p.starts_with(&root));
        // tmp staging is clean after commit
        let tmp_dir = p.parent().unwrap().parent().unwrap().parent().unwrap().join("tmp");
        assert_eq!(fs::read_dir(tmp_dir).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sessions_are_isolated() {
        let (s, root) = store(|_| {});
        let a = s.session(None).unwrap();
        let b = s.session(None).unwrap();
        assert_ne!(a, b);
        let m = s.add_file(&a, "a.rpf", &mut Cursor::new(vec![1, 2, 3])).unwrap();
        assert_eq!(s.original_path(&b, &m.file_id), Err(StoreError::NotFound));
        assert!(s.list_files(&b).unwrap().is_empty());
        // Unknown / malformed tokens never resolve to an existing workspace; they get a fresh one.
        let fresh = s.session(Some("../../etc")).unwrap();
        assert!(valid_token(&fresh) && fresh != a && fresh != b);
        assert_eq!(s.list_files("../../etc"), Err(StoreError::NotFound));
        let guess = "0".repeat(64);
        assert_eq!(s.original_path(&guess, &m.file_id), Err(StoreError::NotFound));
        // A valid token is returned unchanged.
        assert_eq!(s.session(Some(&a)).unwrap(), a);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn enforces_limits_and_cleans_staging() {
        let (s, root) = store(|c| {
            c.max_upload_bytes = 10;
            c.max_workspace_bytes = 15;
            c.max_files_per_workspace = 2;
        });
        let t = s.session(None).unwrap();
        assert_eq!(s.add_file(&t, "big", &mut Cursor::new(vec![0; 11])), Err(StoreError::UploadTooLarge));
        s.add_file(&t, "a", &mut Cursor::new(vec![0; 10])).unwrap();
        assert_eq!(s.add_file(&t, "b", &mut Cursor::new(vec![0; 6])), Err(StoreError::QuotaExceeded));
        s.add_file(&t, "c", &mut Cursor::new(vec![0; 5])).unwrap();
        assert_eq!(s.add_file(&t, "d", &mut Cursor::new(vec![0; 1])), Err(StoreError::TooManyFiles));
        assert_eq!(s.list_files(&t).unwrap().len(), 2);
        // Failed uploads leave no staging files or orphan file dirs.
        let ws_dir = fs::read_dir(&root).unwrap().next().unwrap().unwrap().path();
        assert_eq!(fs::read_dir(ws_dir.join("tmp")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(ws_dir.join("files")).unwrap().count(), 2);
        assert_eq!(s.add_file(&t, "..", &mut Cursor::new(vec![])), Err(StoreError::InvalidName));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_versions_backups_and_quota() {
        let (s, root) = store(|c| c.max_workspace_bytes = 100);
        let t = s.session(None).unwrap();
        let m = s.add_file(&t, "a.rpf", &mut Cursor::new(vec![1; 10])).unwrap();
        assert!(s.current_path(&t, &m.file_id).unwrap().ends_with("original"));
        // First save: original is the previous version and is never touched, so no backup is made.
        assert_eq!(s.save_version(&t, &m.file_id, &[2; 20]).unwrap(), (10, 20, None));
        let cur = s.current_path(&t, &m.file_id).unwrap();
        assert!(cur.ends_with("current"));
        assert_eq!(fs::read(&cur).unwrap(), vec![2; 20]);
        assert_eq!(fs::read(s.original_path(&t, &m.file_id).unwrap()).unwrap(), vec![1; 10]);
        // Second save backs up the previous current.
        let (old, new, b) = s.save_version(&t, &m.file_id, &[3; 30]).unwrap();
        assert_eq!((old, new), (20, 30));
        let bid = b.unwrap();
        let list = s.list_backups(&t, &m.file_id).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(fs::read(s.backup_path(&t, &m.file_id, &bid).unwrap()).unwrap(), vec![2; 20]);
        // Foreign / forged ids and other sessions never resolve.
        assert_eq!(s.backup_path(&t, &m.file_id, "../current"), Err(StoreError::NotFound));
        assert_eq!(s.backup_path(&t, &m.file_id, "999999"), Err(StoreError::NotFound));
        let other = s.session(None).unwrap();
        assert_eq!(s.list_backups(&other, &m.file_id), Err(StoreError::NotFound));
        assert_eq!(s.save_version(&other, &m.file_id, &[0]), Err(StoreError::NotFound));
        // Quota: 10 orig + 30 cur + 20 backup = 60. A 45-byte save needs 10+45+30(backup) = 85 <= 100 OK.
        s.save_version(&t, &m.file_id, &[4; 45]).unwrap();
        // Next 60-byte save would need 10+60+45 + older backups; oldest backups are dropped to fit.
        s.save_version(&t, &m.file_id, &[5; 45]).unwrap();
        // A save that cannot fit even with no backups is refused and leaves current intact.
        assert_eq!(s.save_version(&t, &m.file_id, &[6; 95]), Err(StoreError::QuotaExceeded));
        assert_eq!(fs::read(s.current_path(&t, &m.file_id).unwrap()).unwrap(), vec![5; 45]);
        let ws_dir = fs::read_dir(&root).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).find(|p| p.join("tmp").exists()).unwrap();
        assert_eq!(fs::read_dir(ws_dir.join("tmp")).unwrap().filter(|e| e.is_ok()).count() <= 1, true);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn backup_cap() {
        let (s, root) = store(|_| {});
        let t = s.session(None).unwrap();
        let m = s.add_file(&t, "a", &mut Cursor::new(vec![0])).unwrap();
        for i in 0..(MAX_BACKUPS_PER_FILE + 5) {
            s.save_version(&t, &m.file_id, &[i as u8]).unwrap();
        }
        let list = s.list_backups(&t, &m.file_id).unwrap();
        assert_eq!(list.len(), MAX_BACKUPS_PER_FILE);
        assert!(list[0].id > list[1].id, "newest first");
        let n = fs::read_dir(s.current_path(&t, &m.file_id).unwrap().parent().unwrap().join("backups")).unwrap().count();
        assert_eq!(n, MAX_BACKUPS_PER_FILE, "pruned backups are removed from disk");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_limit() {
        let (s, root) = store(|c| c.max_workspaces = 2);
        s.session(None).unwrap();
        s.session(None).unwrap();
        assert_eq!(s.session(None), Err(StoreError::TooManyWorkspaces));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn delete_and_expiry_remove_data() {
        let (s, root) = store(|c| {
            c.idle_ttl = Duration::from_secs(100);
            c.max_age = Duration::from_secs(1000);
        });
        let a = s.session(None).unwrap();
        let b = s.session(None).unwrap();
        s.add_file(&a, "a", &mut Cursor::new(vec![1])).unwrap();
        s.add_file(&b, "b", &mut Cursor::new(vec![1])).unwrap();
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        s.delete_session(&a);
        assert_eq!(s.list_files(&a), Err(StoreError::NotFound));
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        assert!(s.ready());
        assert!(s.sweep_at(Instant::now() + Duration::from_secs(50)).is_empty());
        assert_eq!(s.sweep_at(Instant::now() + Duration::from_secs(101)), vec![b.clone()]);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        assert_eq!(s.list_files(&b), Err(StoreError::NotFound));
        let _ = fs::remove_dir_all(root);
    }
}
