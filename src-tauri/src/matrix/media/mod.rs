//! P6.4 — Media upload queue foundation (harness).
//!
//! Tracks [`UploadJob`] metadata only — **no file bytes**, no SDK upload,
//! no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.4-media-upload.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod upload_queue;

pub use error::MediaError;
pub use upload_queue::{UploadQueue, MAX_ACTIVE_UPLOADS};

/// Static marker for link / schema smoke.
pub const MATRIX_MEDIA_MARKER: &str = "matrix-media-upload-p6.4";

/// Touch media upload paths so they remain linked in non-test builds.
pub fn matrix_media_markers() -> &'static str {
    let q = UploadQueue::new(0);
    debug_assert!(q.is_empty());
    debug_assert_eq!(MAX_ACTIVE_UPLOADS, 16);
    debug_assert_eq!(MATRIX_MEDIA_MARKER, "matrix-media-upload-p6.4");
    MATRIX_MEDIA_MARKER
}

#[cfg(test)]
mod tests;
