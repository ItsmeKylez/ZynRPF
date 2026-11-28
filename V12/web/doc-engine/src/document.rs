//! One open archive. M2 scope: open, summary, read an entry, serialize+verify.
use std::sync::Arc;

use serde::Serialize;
use tlk_rpf_core::oiv::{self, Paket};
use tlk_rpf_core::rpf::{self, Arsiv, Dugum};

use crate::error::EngineError;
use crate::paths::validate_virtual_path;
use crate::tree::{Node, build_tree, estimated_size};

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub source: String,
    pub archives: Vec<String>,
    pub path: String,
    pub delete: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub targets: Vec<TargetInfo>,
}

impl From<&Paket> for PackageInfo {
    fn from(p: &Paket) -> Self {
        PackageInfo {
            name: p.ad.clone(),
            version: p.surum.clone(),
            author: p.yazar.clone(),
            description: p.aciklama.clone(),
            targets: p
                .hedefler
                .iter()
                .map(|h| TargetInfo { source: h.kaynak.clone(), archives: h.arsivler.clone(), path: h.yol.clone(), delete: h.sil })
                .collect(),
        }
    }
}

/// English-field equivalent of the desktop `BelgeOzet`. In M2 there is no editing, so
/// `dirty` is always false and `undo_depth`/`redo_depth` are always 0; the fields exist so the
/// contract in docs/M1_API_CONTRACT.md does not change when editing arrives.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSummary {
    pub doc_id: u32,
    pub name: String,
    pub encryption: String,
    pub disk_size: u64,
    pub estimated_size: u64,
    pub dirty: bool,
    pub undo_depth: usize,
    pub redo_depth: usize,
    pub revision: u64,
    pub package: Option<PackageInfo>,
    pub package_error: Option<String>,
    pub missing_sources: Vec<String>,
    pub tree: Vec<Node>,
}

pub struct Document {
    pub name: String,
    pub(crate) archive: Arsiv,
    pub(crate) disk_size: u64,
    /// Identifies the current archive state. Undo/redo restore the identifier of the state they return to
    /// (same model as the desktop `durum_no`), so "dirty" = current state is not the last saved state.
    pub(crate) revision: u64,
    pub(crate) next_rev: u64,
    pub(crate) clean_revision: Option<u64>,
    pub(crate) undo: Vec<(u64, Arsiv)>,
    pub(crate) redo: Vec<(u64, Arsiv)>,
}

fn encryption_name(code: u32) -> &'static str {
    match code {
        rpf::SIFRE_OPEN => "OPEN",
        rpf::SIFRE_CFXP => "CFXP",
        rpf::SIFRE_YOK => "NONE",
        // core returns "AES"/"NG"/Turkish "bilinmiyor"; only the first two are meaningful in English.
        other => match rpf::sifreleme_adi(other) {
            "AES" => "AES",
            "NG" => "NG",
            _ => "UNKNOWN",
        },
    }
}

impl Document {
    pub fn open(name: &str, bytes: Arc<[u8]>) -> Result<Document, EngineError> {
        let disk_size = bytes.len() as u64;
        let archive = Arsiv::oku(bytes)?;
        Ok(Document { name: name.to_string(), archive, disk_size, revision: 1, next_rev: 1, clean_revision: Some(1), undo: Vec::new(), redo: Vec::new() })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn package(&self) -> Option<Result<Paket, String>> {
        let f = self.archive.dosya("assembly.xml").ok()?;
        Some(f.veri().map_err(|e| EngineError::from(e).message).and_then(|v| {
            oiv::coz(&String::from_utf8_lossy(&v)).map_err(|e| EngineError::from(e).message)
        }))
    }

    pub fn summary(&self, doc_id: u32) -> DocumentSummary {
        let (package, package_error) = match self.package() {
            Some(Ok(p)) => (Some(p), None),
            Some(Err(e)) => (None, Some(e)),
            None => (None, None),
        };
        let missing_sources = package
            .iter()
            .flat_map(|p| &p.hedefler)
            .filter(|h| !h.sil && !matches!(self.archive.bul(&format!("content/{}", h.kaynak)), Some(Dugum::Dosya(_) | Dugum::Arsiv(_))))
            .map(|h| h.kaynak.clone())
            .collect();
        DocumentSummary {
            doc_id,
            name: self.name.clone(),
            encryption: encryption_name(self.archive.sifreleme).to_string(),
            disk_size: self.disk_size,
            estimated_size: estimated_size(&self.archive.kok),
            dirty: self.is_dirty(),
            undo_depth: self.undo.len(),
            redo_depth: self.redo.len(),
            revision: self.revision,
            package: package.as_ref().map(PackageInfo::from),
            package_error,
            missing_sources,
            tree: build_tree(&self.archive.kok, ""),
        }
    }

    /// Bytes of one file entry (decompressed as stored; resources are returned as stored in the archive).
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, EngineError> {
        let path = validate_virtual_path(path, false)?;
        let file = self.archive.dosya(&path)?;
        Ok(file.veri()?.into_owned())
    }

    /// Serializes the archive and re-parses the result before returning it ("validate rebuilt archive").
    pub fn serialize_verified(&self) -> Result<Vec<u8>, EngineError> {
        let bytes = self.archive.yaz()?;
        Arsiv::oku(Arc::from(bytes.clone())).map_err(|e| {
            let mut err = EngineError::from(e);
            err.code = "VERIFY_FAILED";
            err.message = "The rebuilt archive failed verification and was not saved.".to_string();
            err
        })?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tlk_rpf_core::rpf::Dosya;

    fn sample_bytes() -> Vec<u8> {
        let mut a = Arsiv::yeni();
        let paket = Paket { ad: "Test Mod".into(), yazar: "Tester".into(), surum: "1.0".into(), ..Default::default() };
        a.koy("", Dugum::Dosya(Dosya::yeni("assembly.xml", oiv::uret(&paket).into_bytes()))).unwrap();
        a.dizin_olustur("content").unwrap();
        a.koy("content", Dugum::Dosya(Dosya::yeni("readme.txt", b"hello".to_vec()))).unwrap();
        a.yaz().unwrap()
    }

    #[test]
    fn opens_and_summarizes() {
        let doc = Document::open("test.rpf", Arc::from(sample_bytes())).unwrap();
        let s = doc.summary(7);
        assert_eq!(s.doc_id, 7);
        assert_eq!(s.encryption, "OPEN");
        assert!(!s.dirty && s.undo_depth == 0 && s.redo_depth == 0 && s.revision == 1);
        assert_eq!(s.package.as_ref().map(|p| p.name.as_str()), Some("Test Mod"));
        assert!(s.missing_sources.is_empty());
        let names: Vec<&str> = s.tree.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"assembly.xml") && names.contains(&"content"), "{names:?}");
        let content = s.tree.iter().find(|n| n.name == "content").unwrap();
        assert_eq!(content.path, "content");
        assert_eq!(content.children[0].path, "content/readme.txt");
    }

    #[test]
    fn reads_entry_and_rejects_bad_paths() {
        let doc = Document::open("test.rpf", Arc::from(sample_bytes())).unwrap();
        assert_eq!(doc.read_file("content/readme.txt").unwrap(), b"hello");
        assert_eq!(doc.read_file("content/missing.txt").unwrap_err().code, "NOT_FOUND");
        assert_eq!(doc.read_file("../etc/passwd").unwrap_err().code, "INVALID_PATH");
        assert_eq!(doc.read_file("content").unwrap_err().code, "NOT_FOUND"); // directory, not a file
    }

    #[test]
    fn serialize_round_trip_preserves_content() {
        let doc = Document::open("test.rpf", Arc::from(sample_bytes())).unwrap();
        let out = doc.serialize_verified().unwrap();
        let again = Document::open("again.rpf", Arc::from(out)).unwrap();
        assert_eq!(again.read_file("content/readme.txt").unwrap(), b"hello");
        assert_eq!(again.summary(1).package.unwrap().name, "Test Mod");
    }

    #[test]
    fn garbage_is_rejected_with_stable_code() {
        let err = Document::open("x.rpf", Arc::from(b"not an rpf at all".to_vec())).err().unwrap();
        assert!(["RPF_INVALID", "RPF_CORRUPT"].contains(&err.code), "{err}");
        assert!(!err.message.contains('/'), "message must not leak paths");
    }
}
