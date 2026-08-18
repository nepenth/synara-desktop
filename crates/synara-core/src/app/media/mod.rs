//! P6.4 — Media upload queue foundation (harness).
//! P7.2 — Media download / local-delivery queue foundation (harness).
//!
//! Tracks upload and download job **metadata only** — **no file bytes**, no
//! SDK media network, no production Tauri commands, no dual-backend.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p6.4-media-upload.md`
//! - `docs/matrix-rust-sdk/p7.2-media-download.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod bounded;
mod download_queue;
mod error;
mod ipc;
mod upload_queue;

pub use bounded::{download_media_bounded, BoundedMediaError};
pub use download_queue::{
    DownloadId, DownloadJob, DownloadKind, DownloadQueue, DownloadState, MAX_ACTIVE_DOWNLOADS,
    MAX_MEDIA_ID_CHARS, MAX_TRACKED_DOWNLOADS,
};
pub use error::MediaError;
pub use ipc::{
    MatrixMediaConfigResult, MatrixMediaDownloadRequest, MatrixMediaDownloadResult,
    MatrixUploadMediaResult,
};
pub use upload_queue::{UploadQueue, MAX_ACTIVE_UPLOADS};

/// Static marker for link / schema smoke (upload + download foundations).
pub const MATRIX_MEDIA_MARKER: &str = "matrix-media-upload-p6.4+download-p7.2";

/// Touch media paths so they remain linked in non-test builds.
pub fn matrix_media_markers() -> &'static str {
    let u = UploadQueue::new(0);
    let d = DownloadQueue::new(0);
    debug_assert!(u.is_empty());
    debug_assert!(d.is_empty());
    debug_assert_eq!(MAX_ACTIVE_UPLOADS, 16);
    debug_assert_eq!(MAX_ACTIVE_DOWNLOADS, 32);
    debug_assert_eq!(DownloadKind::Thumbnail.as_str(), "thumbnail");
    debug_assert_eq!(
        MATRIX_MEDIA_MARKER,
        "matrix-media-upload-p6.4+download-p7.2"
    );
    MATRIX_MEDIA_MARKER
}

#[cfg(test)]
mod tests;
