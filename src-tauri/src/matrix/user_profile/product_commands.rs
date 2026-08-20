use super::*;

/// V-SEND.R-AVATAR-UPLOAD — sole native owner for the logged-in user's
/// display name write. Empty string removes the display name (set to `None`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.setDisplayName` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_own_display_name(
    core: State<'_, Arc<synara_core::Core>>,
    display_name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::own_profile::set_own_display_name(core.inner().as_ref(), display_name).await
}

/// V-SEND.R-AVATAR-UPLOAD — sole native owner for the logged-in user's avatar
/// URL write. Empty string removes the avatar (set to `None`). The `mxc` must
/// be a valid `mxc://` URI (typically produced by `matrix_upload_media`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.setAvatarUrl` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_own_avatar(
    core: State<'_, Arc<synara_core::Core>>,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::own_profile::set_own_avatar(core.inner().as_ref(), mxc).await
}

/// Own-profile read from the homeserver. Avatar is an `mxc://` URI only.
#[tauri::command]
pub async fn matrix_get_own_profile(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<synara_core::app::user_profile::MatrixOwnProfile, MatrixAuthCommandError> {
    crate::bridge::own_profile::get_own_profile(core.inner().as_ref()).await
}

pub(super) fn map_avatar_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-send.r-avatar-display-name-empty"
        | "v-send.r-avatar-display-name-too-long"
        | "v-send.r-avatar-invalid-mxc"
        | "v-send.r-avatar-upload-empty"
        | "v-send.r-avatar-upload-invalid-mime"
        | "v-send.r-avatar-upload-too-large" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix profile request is invalid.",
            diagnostic_id,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix profile operation failed.",
            diagnostic_id,
        ),
    }
}

/// Parse and validate a display name. Empty/whitespace-only input is treated as
/// a removal request (`None`). Non-empty names are trimmed and capped.
pub(super) fn parse_display_name(
    display_name: &str,
) -> Result<Option<String>, MatrixAuthCommandError> {
    synara_core::app::user_profile::parse_own_display_name(display_name).map_err(map_avatar_error)
}

/// Parse and validate an avatar MXC URI. Empty/whitespace-only input is treated
/// as a removal request (`None`). Non-empty values must be valid `mxc://` URIs.
pub(super) fn parse_avatar_mxc(mxc: &str) -> Result<Option<OwnedMxcUri>, MatrixAuthCommandError> {
    synara_core::app::user_profile::parse_own_avatar_mxc(mxc).map_err(map_avatar_error)
}
