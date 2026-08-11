//! Media handle DTO — metadata + handles only; **no bytes**, no key material.

use serde::{Deserialize, Serialize};

use super::ids::MediaHandleId;

/// Where the media content is sourced from (product enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSource {
    Mxc,
    LocalCache,
    Upload,
}

impl MediaSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mxc => "mxc",
            Self::LocalCache => "local_cache",
            Self::Upload => "upload",
        }
    }
}

/// Product media handle — references content without embedding bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaHandle {
    pub handle_id: MediaHandleId,
    /// Optional mxc URI string (never resolved bytes on wire).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mxc_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<MediaSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_handle_id: Option<MediaHandleId>,
}
