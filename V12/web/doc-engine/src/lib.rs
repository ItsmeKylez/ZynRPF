//! Tauri-free document layer (design D1 = option A: adapted, not shared, with `src-tauri/src/lib.rs`).
//!
//! Source mapping (for drift tracking), all in `src-tauri/src/lib.rs` at the time of M0:
//! - tree building: `agac_kur`, `dizin_bilgisi`, `tahmini_boyut`  -> `tree.rs`
//! - summary: `ozet`, `paket_oku`                                  -> `document.rs`
//! - save verification: `kaydet` (re-parse of written bytes)        -> `Document::serialize_verified`
//!
//! M2: open, summary/tree, read one entry, serialize+verify. M4: edit operations with undo/redo (`edit.rs`), entry
//! view (`entry.rs`), ZIP export (`export.rs`). No texture/model/map editors yet (M5).

pub mod document;
pub mod edit;
pub mod entry;
pub mod error;
pub mod export;
pub mod paths;
pub mod settings;
pub mod tree;

pub use document::{Document, DocumentSummary, PackageInfo, TargetInfo};
pub use edit::{HISTORY_LIMIT, ImportItem};
pub use entry::EntryView;
pub use settings::{SettingColor, SettingEntry, SettingsView};
pub use error::EngineError;
