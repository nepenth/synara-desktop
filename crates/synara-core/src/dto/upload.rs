//! Media upload job DTO — progress/state only; **no file bytes**.

use serde::{Deserialize, Serialize};

use super::ids::{MediaHandleId, RoomId, UploadId};

/// Upload job lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadState {
    Queued,
    Uploading,
    Completed,
    Failed,
    Cancelled,
}

impl UploadState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Uploading => "uploading",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Upload job projection for send-queue / composer UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadJob {
    pub upload_id: UploadId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,
    pub file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    pub state: UploadState,
    /// Progress in \[0.0, 1.0\] while uploading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress01: Option<f64>,
    /// Set when `state` is `completed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_handle_id: Option<MediaHandleId>,
}
