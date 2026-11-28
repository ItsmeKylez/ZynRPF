//! Zyn-RPF web backend (M2, batch 2a). std-only so it can be compiled and tested without a crate cache.
//! The HTTP layer (routes, cookie handling, streaming) is deliberately absent: it needs new crates (decision D3, unapproved).
pub mod app;
pub mod config;
pub mod store;
