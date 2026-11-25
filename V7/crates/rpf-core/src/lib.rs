//! rpf-core: clean-room RPF7 index parser with strict limits.
//!
//! The on-disk layout implemented here is PARTIAL knowledge of the public format and has NOT been
//! verified against real game archives (see docs: acceptance N). Every declared size is checked
//! against the real file length and the configured limits before any allocation.
pub mod deflate;
pub mod error;
pub mod format;
pub mod index;
pub mod rebuild;
pub mod rsc;
pub mod doku;
pub mod vs;
pub mod ypt;
pub mod ytd;
pub mod validate;
pub mod testgen;

pub use error::{Code, Fail, Severity};
pub use format::{Entry, Kind};
pub use index::{EntryInfo, Index, Limits};

// ---- Copied verbatim from the original Zyn-RPF core lib.rs (GPL-3.0-only, Copyright (c) 2026 Talkdedsec); needed by rsc.rs and ypt.rs ----

#[derive(Debug, thiserror::Error)]
pub enum Hata {
    #[error("RPF7 arşivi değil")]
    RpfDegil,
    #[error("{0} şifreli arşiv, yalnız OPEN/CFXP arşivler düzenlenebilir")]
    Sifreli(&'static str),
    #[error("bozuk veri: {0}")]
    Bozuk(String),
    #[error("bulunamadı: {0}")]
    Yok(String),
    #[error("{0}")]
    Gecersiz(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Sonuc<T> = Result<T, Hata>;

pub(crate) fn bozuk(m: impl Into<String>) -> Hata {
    Hata::Bozuk(m.into())
}

pub(crate) fn u16_oku(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

pub(crate) fn u32_oku(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

pub(crate) fn u64_oku(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(o..o + 8)?.try_into().ok()?))
}

