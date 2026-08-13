//! Credential-free user/room profile write IPC DTO.

use serde::Serialize;

/// V-SEND.R-AVATAR-UPLOAD — result of a native user-profile write
/// (display name or avatar URL). `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixProfileWriteResult {
    pub status: &'static str,
}
