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

/// Upper bound for a content URI accepted on the wire (mirrors the former
/// CallWidget constraint; now the shared general media guard).
pub(super) const MAX_MEDIA_DOWNLOAD_URI_BYTES: usize = 2048;
/// Upper bound for an original-file download returned to the renderer.
pub(super) const MAX_MEDIA_DOWNLOAD_BYTES: usize = 300 * 1024 * 1024;

/// V-SEND.R-MEDIA / SNC-P3.5 — retain the exact zero-argument media-config
/// invoke while routing only its envelope and fixed response serialization
/// through Core. The desktop Platform remains the sole SDK client/session/cache
/// and store owner; it supplies Core a bounded, string-free projection.
#[tauri::command]
pub async fn matrix_media_config(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<MatrixMediaConfigResult, MatrixAuthCommandError> {
    crate::bridge::media_config::media_config(core.inner().as_ref()).await
}

/// V-SEND.R-MEDIA / P4-S36 — original-file download owner.
///
/// Timeline handles (`timeline-media-*`) resolve through the native owner so
/// encrypted sources never become leftover `mxc://` on the wire. Plain `mxc://`
/// stays available for leftover avatar/pack paths only. Bytes never cross
/// `Core::command`.
#[tauri::command]
pub async fn matrix_media_download(
    state: State<'_, MatrixAuthState>,
    content_uri: String,
) -> Result<MatrixMediaDownloadResult, MatrixAuthCommandError> {
    let request = MatrixMediaDownloadRequest { content_uri };
    if crate::matrix::timeline::is_timeline_media_handle(&request.content_uri) {
        return download_timeline_media_handle(&state, &request.content_uri).await;
    }
    let content_uri = parse_media_download_uri(&request.content_uri)?;
    let media_request = MediaRequestParameters {
        source: MediaSource::Plain(content_uri),
        format: MediaFormat::File,
    };
    let client = {
        let session = state.session.lock().await;
        require_session(session.as_ref())?.client.clone()
    };
    let bytes = client
        .media()
        .get_media_content(&media_request, true)
        .await
        .map_err(|_| map_media_download_error("v-send.r-media-download-sdk-failed"))?;
    validate_media_download_size(bytes.len())?;

    Ok(MatrixMediaDownloadResult { bytes })
}

async fn download_timeline_media_handle(
    state: &State<'_, MatrixAuthState>,
    handle: &str,
) -> Result<MatrixMediaDownloadResult, MatrixAuthCommandError> {
    let Some((client, source)) = state.resolve_timeline_media(handle).await else {
        return Err(map_media_download_error("v-send.r-media-unknown-handle"));
    };
    let media_request = MediaRequestParameters {
        source: source.source,
        format: MediaFormat::File,
    };
    let bytes = client
        .media()
        .get_media_content(&media_request, true)
        .await
        .map_err(|_| map_media_download_error("v-send.r-media-download-sdk-failed"))?;
    validate_media_download_size(bytes.len())?;
    Ok(MatrixMediaDownloadResult { bytes })
}

pub(super) fn parse_media_download_uri(
    content_uri: &str,
) -> Result<OwnedMxcUri, MatrixAuthCommandError> {
    if content_uri.is_empty()
        || content_uri.len() > MAX_MEDIA_DOWNLOAD_URI_BYTES
        || content_uri != content_uri.trim()
        || !content_uri.is_ascii()
        || content_uri.contains(['?', '#'])
    {
        return Err(map_media_download_error(
            "v-send.r-media-invalid-content-uri",
        ));
    }

    let owned = OwnedMxcUri::from(content_uri);
    let valid = owned.validate().is_ok()
        && owned
            .media_id()
            .map(|media_id| !media_id.is_empty())
            .unwrap_or(false);
    if !valid {
        return Err(map_media_download_error(
            "v-send.r-media-invalid-content-uri",
        ));
    }
    Ok(owned)
}

pub(super) fn validate_media_download_size(byte_len: usize) -> Result<(), MatrixAuthCommandError> {
    if byte_len > MAX_MEDIA_DOWNLOAD_BYTES {
        return Err(map_media_download_error(
            "v-send.r-media-download-too-large",
        ));
    }
    Ok(())
}

pub(super) fn map_media_download_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let code = match diagnostic_id {
        "v-send.r-media-invalid-content-uri" => "InvalidRequest",
        _ => "Unknown",
    };
    MatrixAuthCommandError::new(
        code,
        "The native media operation is unavailable.",
        diagnostic_id,
    )
}
