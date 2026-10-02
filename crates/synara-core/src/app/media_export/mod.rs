//! P7.5 — Save/share/open/drag media export intent foundation.
//!
//! Tracks metadata-only [`ExportJob`] values. It performs no filesystem,
//! platform share, open, or drag operations and never stores media bytes.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.5-save-share.md`

mod error;
mod queue;

pub use error::ExportError;
pub use queue::{ExportJob, ExportJobId, ExportKind, ExportQueue, ExportState};

#[cfg(test)]
mod tests;
