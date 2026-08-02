use super::*;

/// V-SEND.R-CALL-MEDIA — sole native CallWidget media-config owner.
/// Uses the live managed Matrix SDK client and returns only the widget's exact
/// `m.upload.size` field. Native/session/SDK failures are terminal.
#[tauri::command]
pub async fn matrix_call_media_config(
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixCallMediaConfigResult, MatrixAuthCommandError> {
    let client = {
        let session = state.session.lock().await;
        require_call_widget_media_session(session.as_ref())?
            .client
            .clone()
    };
    let upload_size = client
        .load_or_fetch_max_upload_size()
        .await
        .map_err(|_| map_call_widget_media_error("v-send.r-call-media-config-sdk-failed"))?;
    let upload_size = project_call_media_upload_size(upload_size)?;

    Ok(MatrixCallMediaConfigResult { upload_size })
}

/// V-SEND.R-CALL-MEDIA — sole native CallWidget original-file download owner.
/// The managed SDK media cache may satisfy the request; otherwise the SDK uses
/// the authenticated media endpoint selected for this live client.
#[tauri::command]
pub async fn matrix_media_download(
    state: State<'_, MatrixAuthState>,
    content_uri: String,
) -> Result<MatrixMediaDownloadResult, MatrixAuthCommandError> {
    let request = MatrixMediaDownloadRequest { content_uri };
    let content_uri = parse_call_widget_media_uri(&request.content_uri)?;
    let media_request = MediaRequestParameters {
        source: MediaSource::Plain(content_uri),
        format: MediaFormat::File,
    };
    let client = {
        let session = state.session.lock().await;
        require_call_widget_media_session(session.as_ref())?
            .client
            .clone()
    };
    let bytes = client
        .media()
        .get_media_content(&media_request, true)
        .await
        .map_err(|_| map_call_widget_media_error("v-send.r-call-media-download-sdk-failed"))?;
    validate_call_widget_media_download_size(bytes.len())?;

    Ok(MatrixMediaDownloadResult { bytes })
}

pub(super) fn require_call_widget_media_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| map_call_widget_media_error("v-send.r-call-media-requires-session"))
}

pub(super) fn project_call_media_upload_size(
    upload_size: UInt,
) -> Result<u64, MatrixAuthCommandError> {
    // `UInt` is already JS-safe, but keep the product boundary explicit so a
    // future SDK type change cannot silently round a value on the wire.
    let upload_size = u64::try_from(i64::from(upload_size))
        .map_err(|_| map_call_widget_media_error("v-send.r-call-media-config-unsafe-size"))?;
    if upload_size > MAX_WIRE_COUNTER {
        return Err(map_call_widget_media_error(
            "v-send.r-call-media-config-unsafe-size",
        ));
    }
    Ok(upload_size)
}

pub(super) fn parse_call_widget_media_uri(
    content_uri: &str,
) -> Result<OwnedMxcUri, MatrixAuthCommandError> {
    if content_uri.is_empty()
        || content_uri.len() > MAX_CALL_WIDGET_MEDIA_URI_BYTES
        || content_uri != content_uri.trim()
        || !content_uri.is_ascii()
        || content_uri.contains(['?', '#'])
    {
        return Err(map_call_widget_media_error(
            "v-send.r-call-media-invalid-content-uri",
        ));
    }

    let owned = OwnedMxcUri::from(content_uri);
    let valid = owned.validate().is_ok()
        && owned
            .media_id()
            .map(|media_id| !media_id.is_empty())
            .unwrap_or(false);
    if !valid {
        return Err(map_call_widget_media_error(
            "v-send.r-call-media-invalid-content-uri",
        ));
    }
    Ok(owned)
}

pub(super) fn validate_call_widget_media_download_size(
    byte_len: usize,
) -> Result<(), MatrixAuthCommandError> {
    if byte_len > MAX_CALL_WIDGET_MEDIA_DOWNLOAD_BYTES {
        return Err(map_call_widget_media_error(
            "v-send.r-call-media-download-too-large",
        ));
    }
    Ok(())
}

pub(super) fn map_call_widget_media_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let code = match diagnostic_id {
        "v-send.r-call-media-invalid-content-uri" => "InvalidRequest",
        "v-send.r-call-media-requires-session" => "Forbidden",
        _ => "Unknown",
    };
    MatrixAuthCommandError::new(
        code,
        "The native CallWidget media operation is unavailable.",
        diagnostic_id,
    )
}
