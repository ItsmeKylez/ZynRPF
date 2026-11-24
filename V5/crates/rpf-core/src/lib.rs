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
pub mod ypt;
pub mod validate;
pub mod testgen;

pub use error::{Code, Fail, Severity};
pub use format::{Entry, Kind};
pub use index::{EntryInfo, Index, Limits};
