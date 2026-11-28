//! Editing operations with the desktop's history semantics (`src-tauri/src/lib.rs`: `Belge::degisiklik`, `degistir`,
//! `geri_al`, `yinele`, `kirli`): every edit snapshots the archive first; a failed edit is rolled back WITHOUT touching
//! the history; a successful edit clears redo; history is capped at 60 steps.

use crate::document::Document;
use crate::error::EngineError;
use crate::paths::{prune_nested, validate_entry_name, validate_virtual_path};
use std::sync::Arc;
use tlk_rpf_core::rpf::{self, Arsiv, Dosya, Dugum, IcArsiv};

pub const HISTORY_LIMIT: usize = 60;

/// One uploaded file to import. `rel_path` may contain `/` to import a folder structure (e.g. `skins/a/b.ytd`).
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub rel_path: String,
    pub data: Vec<u8>,
}

fn node_from_bytes(name: &str, data: Vec<u8>) -> Dugum {
    // As on desktop: a `.rpf` that parses becomes a nested archive; one that does not stays a plain file.
    if name.to_lowercase().ends_with(".rpf")
        && let Ok(arsiv) = Arsiv::oku(Arc::from(data.clone()))
    {
        return Dugum::Arsiv(IcArsiv { ad: rpf::ad_duzelt(name), arsiv });
    }
    Dugum::Dosya(Dosya::yeni(name, data))
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() { name.to_string() } else { format!("{dir}/{name}") }
}

fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

impl Document {
    pub fn is_dirty(&self) -> bool {
        self.clean_revision != Some(self.revision)
    }

    /// Optimistic-concurrency check for mutating requests: `None` skips the check (callers that did not send a revision).
    pub fn ensure_revision(&self, expected: Option<u64>) -> Result<(), EngineError> {
        match expected {
            Some(r) if r != self.revision => Err(EngineError::new(
                "STALE_DOCUMENT",
                "The document changed since you last loaded it. Refresh and try again.",
            )),
            _ => Ok(()),
        }
    }

    fn fresh_revision(&mut self) -> u64 {
        self.next_rev += 1;
        self.next_rev
    }

    /// Runs `f` as one undoable step. On error the archive is restored and history (including redo) is untouched.
    pub(crate) fn edit<T>(&mut self, f: impl FnOnce(&mut Arsiv) -> Result<T, EngineError>) -> Result<T, EngineError> {
        let before = self.archive.clone();
        match f(&mut self.archive) {
            Err(e) => {
                self.archive = before;
                Err(e)
            }
            Ok(v) => {
                self.undo.push((self.revision, before));
                if self.undo.len() > HISTORY_LIMIT {
                    self.undo.remove(0);
                }
                self.revision = self.fresh_revision();
                self.redo.clear();
                Ok(v)
            }
        }
    }

    pub fn undo(&mut self) -> bool {
        let Some((rev, previous)) = self.undo.pop() else { return false };
        let current = std::mem::replace(&mut self.archive, previous);
        let current_rev = std::mem::replace(&mut self.revision, rev);
        self.redo.push((current_rev, current));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some((rev, next)) = self.redo.pop() else { return false };
        let current = std::mem::replace(&mut self.archive, next);
        let current_rev = std::mem::replace(&mut self.revision, rev);
        self.undo.push((current_rev, current));
        true
    }

    /// Replaces the content of an existing text/binary file, or a nested archive (parsed as RPF).
    pub fn replace_file(&mut self, path: &str, data: Vec<u8>) -> Result<(), EngineError> {
        let path = validate_virtual_path(path, false)?;
        self.edit(|a| {
            if let Some(Dugum::Arsiv(_)) = a.bul(&path) {
                let inner = Arsiv::oku(Arc::from(data))?;
                let (parent, name) = split_parent(&path);
                return Ok(a.koy(parent, Dugum::Arsiv(IcArsiv { ad: name.to_string(), arsiv: inner }))?);
            }
            a.dosya_mut(&path)?.degistir(data);
            Ok(())
        })
    }

    pub fn write_text(&mut self, path: &str, text: String) -> Result<(), EngineError> {
        self.replace_file(path, text.into_bytes())
    }

    /// Creates a new text file (or replaces one with the same name, as the desktop `koy` does).
    pub fn create_text(&mut self, path: &str, text: String) -> Result<(), EngineError> {
        let path = validate_virtual_path(path, false)?;
        let (parent, name) = split_parent(&path);
        validate_entry_name(name)?;
        self.edit(|a| Ok(a.koy(parent, Dugum::Dosya(Dosya::yeni(name, text.into_bytes())))?))
    }

    /// Adds uploaded files (optionally with folder structure) under `dir` as ONE undo step.
    pub fn import(&mut self, dir: &str, items: Vec<ImportItem>) -> Result<usize, EngineError> {
        let dir = validate_virtual_path(dir, true)?;
        if items.is_empty() {
            return Err(EngineError::new("INVALID_REQUEST", "Nothing to import."));
        }
        let mut parsed = Vec::with_capacity(items.len());
        for it in items {
            let rel = validate_virtual_path(&it.rel_path, false)?;
            let (parents, name) = split_parent(&rel);
            validate_entry_name(name)?;
            parsed.push((parents.to_string(), name.to_string(), it.data));
        }
        let count = parsed.len();
        self.edit(|a| {
            for (parents, name, data) in parsed {
                let mut cur = dir.clone();
                for seg in parents.split('/').filter(|s| !s.is_empty()) {
                    cur = join(&cur, seg);
                    match a.bul(&cur) {
                        Some(Dugum::Dizin(_) | Dugum::Arsiv(_)) => {}
                        Some(Dugum::Dosya(_)) => {
                            return Err(EngineError::new("CONFLICT", "A file with the same name as a folder already exists."));
                        }
                        None => a.dizin_olustur(&cur)?,
                    }
                }
                a.koy(&cur, node_from_bytes(&name, data))?;
            }
            Ok(())
        })?;
        Ok(count)
    }

    pub fn delete(&mut self, paths: Vec<String>) -> Result<(), EngineError> {
        let paths = self.clean_paths(paths)?;
        self.edit(|a| {
            for p in &paths {
                a.sil(p)?;
            }
            Ok(())
        })
    }

    pub fn rename(&mut self, path: &str, new_name: &str) -> Result<(), EngineError> {
        let path = validate_virtual_path(path, false)?;
        let new_name = validate_entry_name(new_name)?;
        self.edit(|a| Ok(a.yeniden_adlandir(&path, &new_name)?))
    }

    pub fn mkdir(&mut self, path: &str) -> Result<(), EngineError> {
        let path = validate_virtual_path(path, false)?;
        validate_entry_name(split_parent(&path).1)?;
        self.edit(|a| Ok(a.dizin_olustur(&path)?))
    }

    /// Moves entries into `dir` ("" = archive root). A folder can never be moved into itself.
    pub fn move_to(&mut self, paths: Vec<String>, dir: &str) -> Result<(), EngineError> {
        let paths = self.clean_paths(paths)?;
        let dir = validate_virtual_path(dir, true)?;
        for p in &paths {
            let inside = format!("{}/", dir.to_lowercase());
            if dir.eq_ignore_ascii_case(p) || inside.starts_with(&format!("{}/", p.to_lowercase())) {
                return Err(EngineError::new("INVALID_REQUEST", "A folder cannot be moved into itself."));
            }
        }
        self.edit(|a| {
            for p in &paths {
                let node = a.sil(p)?;
                a.koy(&dir, node)?;
            }
            Ok(())
        })
    }

    fn clean_paths(&self, paths: Vec<String>) -> Result<Vec<String>, EngineError> {
        if paths.is_empty() {
            return Err(EngineError::new("INVALID_REQUEST", "No entries were selected."));
        }
        let valid = paths.iter().map(|p| validate_virtual_path(p, false)).collect::<Result<Vec<_>, _>>()?;
        Ok(prune_nested(valid))
    }

    /// Call after the server has durably written `bytes` (already verified). The bytes become the saved baseline;
    /// history is kept (as on desktop) so a save can still be undone in the editor.
    pub fn mark_saved(&mut self, bytes: Arc<[u8]>) -> Result<(), EngineError> {
        let size = bytes.len() as u64;
        self.archive = Arsiv::oku(bytes)?;
        self.disk_size = size;
        self.clean_revision = Some(self.revision);
        Ok(())
    }

    /// Replaces the whole document with `bytes` (e.g. a restored backup). History is cleared, state is clean.
    pub fn reload(&mut self, bytes: Arc<[u8]>) -> Result<(), EngineError> {
        let size = bytes.len() as u64;
        let archive = Arsiv::oku(bytes)?;
        self.archive = archive;
        self.disk_size = size;
        self.undo.clear();
        self.redo.clear();
        self.revision = self.fresh_revision();
        self.clean_revision = Some(self.revision);
        Ok(())
    }
}
