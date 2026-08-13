//! Credential-free media IPC request/result DTOs.
//!
//! Live Client media I/O and host file bytes stay in the desktop shell.

use serde::{Deserialize, Serialize};

/// V-SEND.R-AVATAR-UPLOAD — result of a native media upload for a user
/// avatar. Returns the homeserver `mxc://` URI; no file bytes cross back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixUploadMediaResult {
    pub mxc: String,
}

/// V-SEND.R-MEDIA — the exact media-config result shape (`m.upload.size`).
/// This is the React-facing product DTO, not the P2 `PlatformMediaConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixMediaConfigResult {
    #[serde(rename = "m.upload.size")]
    pub upload_size: u64,
}

/// V-SEND.R-MEDIA — original-file bytes returned by the native media owner.
/// This DTO is intentionally not part of a versioned Matrix envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixMediaDownloadResult {
    pub bytes: Vec<u8>,
}

/// V-SEND.R-MEDIA — camelCase request used by the native media owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixMediaDownloadRequest {
    pub content_uri: String,
}
