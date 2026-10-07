//! P6.4 — Media upload queue foundation (harness).
//! P7.2 — Media download / local-delivery queue foundation (harness).
//!
//! Tracks upload and download job **metadata only** — **no file bytes**, no
//! SDK media network, no production Tauri commands, no dual-backend.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p6.4-media-upload.md`
//! - `docs/matrix-rust-sdk/p7.2-media-download.md`

mod bounded;
mod cache;
mod content;
mod download_queue;
mod error;
mod ipc;
mod plain;
mod preview;
mod upload_queue;

pub use bounded::{download_media_bounded, BoundedMediaError};
pub use cache::{
    clear_hot_media_cache, fetch_media_cached, HotMediaCache, HOT_MEDIA_CACHE_MAX_BYTES,
    HOT_MEDIA_CACHE_MAX_ITEM_BYTES,
};
pub use content::{
    parse_content_upload_mime, upload_content, validate_content_upload_filename,
    MAX_CONTENT_UPLOAD_BYTES,
};
pub use download_queue::{
    DownloadId, DownloadJob, DownloadKind, DownloadQueue, DownloadState, MAX_ACTIVE_DOWNLOADS,
    MAX_MEDIA_ID_CHARS, MAX_TRACKED_DOWNLOADS,
};
pub use error::MediaError;
pub use ipc::{
    MatrixMediaConfigResult, MatrixMediaDownloadRequest, MatrixMediaDownloadResult,
    MatrixUploadMediaResult,
};
pub use plain::{
    download_plain_media, parse_plain_media_uri, thumbnail_plain_media,
    MAX_PLAIN_MEDIA_DOWNLOAD_BYTES, MAX_PLAIN_MEDIA_URI_BYTES,
};
pub use preview::{
    map_open_graph_preview, media_preview_gate, preview_url_is_allowed, room_media_preview,
    skipped_media_preview, MappedOpenGraphPreview, MatrixMediaPreviewSnapshot, MediaPreviewCache,
    MediaPreviewGate, MAX_MEDIA_PREVIEW_CACHE, MAX_MEDIA_PREVIEW_URL_BYTES,
};
pub use upload_queue::{UploadQueue, MAX_ACTIVE_UPLOADS};

#[cfg(test)]
mod tests;
