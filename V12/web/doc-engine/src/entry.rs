//! Entry content view (port of `dosya_oku` in `src-tauri/src/lib.rs`, with English kind names).

use crate::document::Document;
use crate::error::EngineError;
use crate::paths::validate_virtual_path;
use serde::Serialize;
use tlk_rpf_core::model;

/// Files larger than this are shown as a hex preview even if they are valid UTF-8, to keep JSON responses bounded.
pub const MAX_TEXT_VIEW_BYTES: usize = 8 * 1024 * 1024;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryView {
    /// `meta | texture | model | resource | setting | text | binary`
    pub kind: &'static str,
    pub size: usize,
    pub text: Option<String>,
    pub hex_dump: Option<String>,
    pub rsc_version: Option<u32>,
}

fn is_text(v: &[u8]) -> bool {
    let head = &v[..v.len().min(16384)];
    !head.contains(&0) && std::str::from_utf8(v).is_ok()
}

pub fn hex_dump(v: &[u8]) -> String {
    let mut s = String::new();
    for (i, row) in v.chunks(16).take(64).enumerate() {
        s.push_str(&format!("{:08x}  ", i * 16));
        for b in row {
            s.push_str(&format!("{b:02x} "));
        }
        s.push_str(&"   ".repeat(16 - row.len()));
        s.push(' ');
        s.extend(row.iter().map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' }));
        s.push('\n');
    }
    s
}

impl Document {
    pub fn entry_view(&self, path: &str) -> Result<EntryView, EngineError> {
        let path = validate_virtual_path(path, false)?;
        let file = self.archive.dosya(&path)?;
        let v = file.veri()?;
        let lower = path.to_lowercase();
        let ends = |exts: &[&str]| exts.iter().any(|e| lower.ends_with(e));
        let size = v.len();
        if ends(&[".ymap", ".ytyp", ".ymt", ".ymf", ".pso"]) && size <= MAX_TEXT_VIEW_BYTES {
            if let Ok(xml) = model::meta_xml(&v) {
                return Ok(EntryView { kind: "meta", size, text: Some(xml), hex_dump: None, rsc_version: None });
            }
        }
        if file.kaynak_mi() {
            let version = u32::from_le_bytes(v.get(4..8).and_then(|x| x.try_into().ok()).unwrap_or([0; 4]));
            let kind = if ends(&[".ytd"]) {
                "texture"
            } else if ends(&[".ydr", ".ydd", ".yft"]) {
                "model"
            } else {
                "resource"
            };
            return Ok(EntryView { kind, size, text: None, hex_dump: Some(hex_dump(&v)), rsc_version: Some(version) });
        }
        if size <= MAX_TEXT_VIEW_BYTES && is_text(&v) {
            let kind = if ends(&["visualsettings.dat"]) { "setting" } else { "text" };
            return Ok(EntryView { kind, size, text: Some(String::from_utf8_lossy(&v).into_owned()), hex_dump: None, rsc_version: None });
        }
        Ok(EntryView { kind: "binary", size, text: None, hex_dump: Some(hex_dump(&v)), rsc_version: None })
    }
}
