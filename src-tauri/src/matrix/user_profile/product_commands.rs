use super::*;

/// V-SEND.R-AVATAR-UPLOAD — sole native owner for the logged-in user's
/// display name write. Empty string removes the display name (set to `None`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.setDisplayName` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_own_display_name(
    state: State<'_, MatrixAuthState>,
    display_name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let display_name = parse_display_name(&display_name)?;
    let client = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.clone()
    };
    client
        .account()
        .set_display_name(display_name.as_deref())
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix display name could not be updated.",
                "v-send.r-avatar-display-name-sdk-failed",
            )
        })?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

/// V-SEND.R-AVATAR-UPLOAD — sole native owner for the logged-in user's avatar
/// URL write. Empty string removes the avatar (set to `None`). The `mxc` must
/// be a valid `mxc://` URI (typically produced by `matrix_upload_media`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.setAvatarUrl` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_own_avatar(
    state: State<'_, MatrixAuthState>,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let mxc = parse_avatar_mxc(&mxc)?;
    let client = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.clone()
    };
    client
        .account()
        .set_avatar_url(mxc.as_deref())
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix avatar could not be updated.",
                "v-send.r-avatar-set-sdk-failed",
            )
        })?;
    Ok(MatrixProfileWriteResult { status: "ok" })
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
    let trimmed = display_name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > 255 {
        return Err(map_avatar_error("v-send.r-avatar-display-name-too-long"));
    }
    Ok(Some(trimmed.to_owned()))
}

/// Parse and validate an avatar MXC URI. Empty/whitespace-only input is treated
/// as a removal request (`None`). Non-empty values must be valid `mxc://` URIs.
pub(super) fn parse_avatar_mxc(mxc: &str) -> Result<Option<OwnedMxcUri>, MatrixAuthCommandError> {
    let trimmed = mxc.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !trimmed.starts_with("mxc://") {
        return Err(map_avatar_error("v-send.r-avatar-invalid-mxc"));
    }
    let owned = OwnedMxcUri::from(trimmed);
    // Reject obviously incomplete URIs (no media id).
    if owned.as_str().matches('/').count() < 3 {
        return Err(map_avatar_error("v-send.r-avatar-invalid-mxc"));
    }
    Ok(Some(owned))
}
