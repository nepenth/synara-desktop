//! P7.5 — Save/share/open/drag media export intent foundation.
//!
//! Tracks metadata-only [`ExportJob`] values. It performs no filesystem,
//! platform share, open, or drag operations and never stores media bytes.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.5-save-share.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod queue;

pub use error::ExportError;
pub use queue::{ExportJob, ExportJobId, ExportKind, ExportQueue, ExportState};

/// Static marker for link/schema smoke checks.
pub const MATRIX_MEDIA_EXPORT_MARKER: &str = "matrix-media-export-p7.5";

/// Touch media export paths so they remain linked in non-test builds.
pub fn matrix_media_export_markers() -> &'static str {
    let queue = ExportQueue::new(0);
    let kind = ExportKind::Save;
    debug_assert!(queue.is_empty());
    debug_assert!(matches!(kind, ExportKind::Save));
    debug_assert_eq!(MATRIX_MEDIA_EXPORT_MARKER, "matrix-media-export-p7.5");
    MATRIX_MEDIA_EXPORT_MARKER
}

#[cfg(test)]
mod tests;
