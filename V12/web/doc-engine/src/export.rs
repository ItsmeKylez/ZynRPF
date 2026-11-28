//! ZIP export of selected entries (replaces desktop `disa_aktar`, which wrote files into a chosen folder).
//! Built in memory and bounded by `max_bytes` of uncompressed content.

use crate::document::Document;
use crate::error::EngineError;
use crate::paths::{prune_nested, validate_virtual_path};
use std::io::{Cursor, Write};
use tlk_rpf_core::rpf::Dugum;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/// One path component made safe for a ZIP entry name (names inside an RPF are untrusted input).
fn safe_component(name: &str) -> String {
    let c: String = name.chars().map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c }).collect();
    match c.trim() {
        "" | "." | ".." => "_".to_string(),
        t => t.to_string(),
    }
}

struct Sink {
    zip: ZipWriter<Cursor<Vec<u8>>>,
    opts: SimpleFileOptions,
    written: u64,
    max: u64,
    files: usize,
}

fn too_large() -> EngineError {
    EngineError::new("EXPORT_TOO_LARGE", "The selection is too large to export in one download. Select fewer files.")
}

fn zip_err(_: impl std::fmt::Debug) -> EngineError {
    EngineError::new("EXPORT_FAILED", "The ZIP file could not be created.")
}

impl Sink {
    fn add(&mut self, prefix: &str, node: &Dugum) -> Result<(), EngineError> {
        let name = format!("{prefix}{}", safe_component(node.ad()));
        match node {
            Dugum::Dosya(f) => {
                let data = f.veri()?;
                self.put(&name, &data)
            }
            Dugum::Arsiv(a) => {
                let data = a.arsiv.yaz()?;
                self.put(&name, &data)
            }
            Dugum::Dizin(d) => {
                self.zip.add_directory(format!("{name}/"), self.opts).map_err(zip_err)?;
                for c in &d.cocuklar {
                    self.add(&format!("{name}/"), c)?;
                }
                Ok(())
            }
        }
    }

    fn put(&mut self, name: &str, data: &[u8]) -> Result<(), EngineError> {
        self.written += data.len() as u64;
        if self.written > self.max {
            return Err(too_large());
        }
        self.zip.start_file(name, self.opts).map_err(zip_err)?;
        self.zip.write_all(data).map_err(zip_err)?;
        self.files += 1;
        Ok(())
    }
}

impl Document {
    /// Exports the given entries (folders recursively; `""` = everything) as a ZIP. Returns `(bytes, file_count)`.
    pub fn export_zip(&self, paths: Vec<String>, max_bytes: u64) -> Result<(Vec<u8>, usize), EngineError> {
        if paths.is_empty() {
            return Err(EngineError::new("INVALID_REQUEST", "No entries were selected."));
        }
        let mut sink = Sink {
            zip: ZipWriter::new(Cursor::new(Vec::new())),
            opts: SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            written: 0,
            max: max_bytes,
            files: 0,
        };
        if paths.iter().any(|p| p.is_empty()) {
            for c in &self.archive.kok.cocuklar {
                sink.add("", c)?;
            }
        } else {
            let valid = paths.iter().map(|p| validate_virtual_path(p, false)).collect::<Result<Vec<_>, _>>()?;
            for p in prune_nested(valid) {
                let node = self.archive.bul(&p).ok_or_else(|| EngineError::new("NOT_FOUND", "The requested entry was not found."))?;
                sink.add("", node)?;
            }
        }
        let files = sink.files;
        let bytes = sink.zip.finish().map_err(zip_err)?.into_inner();
        Ok((bytes, files))
    }
}
