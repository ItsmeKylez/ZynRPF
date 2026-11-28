use std::fmt;
use tlk_rpf_core::Hata;

/// Stable, English, path-free error. `core_message` keeps the original (Turkish) core text for server logs only;
/// the HTTP layer must not return it to clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError {
    pub code: &'static str,
    pub message: String,
    pub core_message: Option<String>,
}

impl EngineError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), core_message: None }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EngineError {}

impl From<Hata> for EngineError {
    fn from(e: Hata) -> Self {
        let core = Some(e.to_string());
        let (code, message) = match &e {
            Hata::RpfDegil => ("RPF_INVALID", "This is not a valid RPF7 archive.".to_string()),
            Hata::Sifreli(kind) => (
                "RPF_ENCRYPTED",
                format!("This archive uses {kind} encryption. Only OPEN, CFXP and unencrypted archives can be opened."),
            ),
            Hata::Bozuk(_) => ("RPF_CORRUPT", "The archive data is corrupt or truncated.".to_string()),
            Hata::Yok(_) => ("NOT_FOUND", "The requested entry was not found.".to_string()),
            Hata::Gecersiz(_) => ("INVALID_REQUEST", "The request could not be applied to this archive.".to_string()),
            // Io may embed server paths: never forward its text.
            Hata::Io(_) => ("IO_ERROR", "A storage error occurred.".to_string()),
        };
        EngineError { code, message, core_message: core }
    }
}
