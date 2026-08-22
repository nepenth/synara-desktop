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

/// Live ignored-user list. User ids only — no tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixIgnoredUsersSnapshot {
    pub user_ids: Vec<String>,
}

/// Result of ignore/unignore. `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixIgnoredUsersWriteResult {
    pub status: &'static str,
}

/// Homeserver-attached email addresses. Addresses only — no tokens, no
/// `client_secret`, no session ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixThreepidSnapshot {
    pub emails: Vec<MatrixThreepidEmail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixThreepidEmail {
    pub address: String,
}

/// Result of a 3PID delete. `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixThreepidWriteResult {
    pub status: &'static str,
}

/// Result of requesting an email token. `session_id` is the homeserver `sid`.
/// The `client_secret` stays in the owner and never crosses this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixThreepidEmailTokenResult {
    pub session_id: String,
}

/// Result of adding an email. `"ok"` when the homeserver accepted the 3PID.
/// `"authenticationRequired"` when password UIAA is needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixThreepidAddResult {
    pub status: String,
}

/// Result of uploading own-avatar bytes. `mxc` is the content URI only —
/// never raw bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixUploadAvatarResult {
    pub mxc: String,
}

/// One user-directory hit. User id / display name may appear; avatar is
/// `mxc://` only — never bytes or `data:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixUserDirectoryHit {
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// Homeserver user-directory search. Hits are metadata only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixUserDirectorySearchResult {
    pub limited: bool,
    pub results: Vec<MatrixUserDirectoryHit>,
}
