//! Credential-free user/room profile IPC DTOs.

use serde::{Deserialize, Serialize};

/// V-SEND.R-AVATAR-UPLOAD — result of a native user-profile write
/// (display name or avatar URL). `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixProfileWriteResult {
    pub status: &'static str,
}

/// Homeserver own-profile read. Avatar is an `mxc://` URI only — never bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixOwnProfile {
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}
