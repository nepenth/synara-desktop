use super::*;

/// V-SEND.R-AVATAR-UPLOAD — sole native owner for user-avatar media upload.
/// Bytes cross IPC once; the SDK `Media::upload` returns the `mxc://` URI which
/// is then passed to `matrix_set_own_avatar`. Reuses the byte-IPC + size-guard
/// pattern of `matrix_send_attachment` (no JS `mx.uploadContent`).
#[tauri::command]
pub async fn matrix_upload_media(
    state: State<'_, MatrixAuthState>,
    mime_type: String,
    bytes: Vec<u8>,
) -> Result<MatrixUploadMediaResult, MatrixAuthCommandError> {
    let mime_type = validate_avatar_mime(&mime_type)?;
    if bytes.is_empty() {
        return Err(map_avatar_error("v-send.r-avatar-upload-empty"));
    }
    if bytes.len() > MAX_AVATAR_IPC_BYTES {
        return Err(map_avatar_error("v-send.r-avatar-upload-too-large"));
    }
    let client = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.clone()
    };
    let response = client
        .media()
        .upload(&mime_type, bytes, None)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix avatar upload failed.",
                "v-send.r-avatar-upload-sdk-failed",
            )
        })?;
    Ok(MatrixUploadMediaResult {
        mxc: response.content_uri.to_string(),
    })
}

/// Validate an avatar upload MIME type. Only image types are accepted for
/// avatars (matching the `image/*` file picker in `Profile.tsx`).
pub(super) fn validate_avatar_mime(mime_type: &str) -> Result<Mime, MatrixAuthCommandError> {
    let mime_type = mime_type.trim();
    if mime_type.is_empty() || mime_type.len() > 255 {
        return Err(map_avatar_error("v-send.r-avatar-upload-invalid-mime"));
    }
    let parsed = mime_type
        .parse::<Mime>()
        .map_err(|_| map_avatar_error("v-send.r-avatar-upload-invalid-mime"))?;
    if parsed.type_() != mime::IMAGE {
        return Err(map_avatar_error("v-send.r-avatar-upload-invalid-mime"));
    }
    Ok(parsed)
}
