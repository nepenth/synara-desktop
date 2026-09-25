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

/// Same presentation budget as the native markdown preview.
pub(super) const MAX_MEDIA_TEXT_PREVIEW_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMediaSaveResult {
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMediaTextPreviewResult {
    pub text: String,
}

/// Save a timeline handle or plain `mxc://` into Downloads.
///
/// The webview receives the saved filename only. Decrypted bytes stay in Rust.
#[tauri::command]
pub async fn matrix_media_save(
    state: State<'_, MatrixAuthState>,
    content_uri: String,
    filename: String,
) -> Result<MatrixMediaSaveResult, MatrixAuthCommandError> {
    let bytes = download_content_bytes(&state, &content_uri, MAX_MEDIA_DOWNLOAD_BYTES).await?;
    let filename = crate::desktop_file_transfer::save_exclusive_download(&filename, &bytes)
        .map_err(|_| map_media_download_error("v-send.r-media-save-failed"))?;
    Ok(MatrixMediaSaveResult { filename })
}

/// UTF-8 preview for markdown and other text the UI has to render.
///
/// Files above [`MAX_MEDIA_TEXT_PREVIEW_BYTES`] fail closed and are not returned.
#[tauri::command]
pub async fn matrix_media_text_preview(
    state: State<'_, MatrixAuthState>,
    content_uri: String,
) -> Result<MatrixMediaTextPreviewResult, MatrixAuthCommandError> {
    let bytes = download_content_bytes(&state, &content_uri, MAX_MEDIA_TEXT_PREVIEW_BYTES).await?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| map_media_download_error("v-send.r-media-preview-not-text"))?;
    Ok(MatrixMediaTextPreviewResult {
        text: text.to_owned(),
    })
}

/// Read-only URL preview. Encrypted and unknown rooms skip the homeserver call.
#[tauri::command]
pub async fn matrix_media_preview(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    session_generation: u64,
    url: String,
    ts: Option<u64>,
) -> Result<MatrixMediaPreviewSnapshot, MatrixAuthCommandError> {
    crate::bridge::media_preview::media_preview(
        core.inner().as_ref(),
        room_id,
        session_generation,
        url,
        ts,
    )
    .await
}

async fn download_content_bytes(
    state: &State<'_, MatrixAuthState>,
    content_uri: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, MatrixAuthCommandError> {
    let too_large_id = if max_bytes == MAX_MEDIA_TEXT_PREVIEW_BYTES {
        "v-send.r-media-preview-too-large"
    } else {
        "v-send.r-media-download-too-large"
    };
    if crate::matrix::timeline::is_timeline_media_handle(content_uri) {
        let Some((client, source)) = state.resolve_timeline_media(content_uri).await else {
            return Err(map_media_download_error("v-send.r-media-unknown-handle"));
        };
        let media_request = MediaRequestParameters {
            source: source.source,
            format: MediaFormat::File,
        };
        return download_bounded(&client, &media_request, max_bytes, too_large_id).await;
    }
    let content_uri = parse_media_download_uri(content_uri)?;
    let media_request = MediaRequestParameters {
        source: MediaSource::Plain(content_uri),
        format: MediaFormat::File,
    };
    let client = {
        let session = state.session.lock().await;
        require_session(session.as_ref())?.client.clone()
    };
    download_bounded(&client, &media_request, max_bytes, too_large_id).await
}

async fn download_bounded(
    client: &matrix_sdk::Client,
    media_request: &MediaRequestParameters,
    max_bytes: usize,
    too_large_id: &'static str,
) -> Result<Vec<u8>, MatrixAuthCommandError> {
    synara_core::app::media::download_media_bounded(client, media_request, max_bytes)
        .await
        .map_err(|error| match error {
            synara_core::app::media::BoundedMediaError::TooLarge => {
                map_media_download_error(too_large_id)
            }
            _ => map_media_download_error("v-send.r-media-download-sdk-failed"),
        })
}

pub(crate) fn parse_media_download_uri(
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
