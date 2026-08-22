//! Live generic content upload. Bytes stay method arguments, never Core JSON.

use matrix_sdk::Client;
use mime::Mime;

use super::MatrixUploadMediaResult;

/// Same IPC cap as desktop composer attachments (`MAX_ATTACHMENT_IPC_BYTES`).
pub const MAX_CONTENT_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

pub fn parse_content_upload_mime(mime_type: &str) -> Result<Mime, &'static str> {
    let mime_type = mime_type.trim();
    if mime_type.is_empty() || mime_type.len() > 255 {
        return Err("v-send.r-content-upload-invalid-mime");
    }
    mime_type
        .parse::<Mime>()
        .map_err(|_| "v-send.r-content-upload-invalid-mime")
}

pub fn validate_content_upload_filename(filename: &str) -> Result<&str, &'static str> {
    let filename = filename.trim();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err("v-send.r-content-upload-invalid-filename");
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
        return Err("v-send.r-content-upload-invalid-filename");
    }
    Ok(filename)
}

pub async fn upload_content(
    client: &Client,
    payload: Vec<u8>,
    mime_type: &str,
    filename: Option<&str>,
) -> Result<MatrixUploadMediaResult, &'static str> {
    let _ = client
        .user_id()
        .ok_or("v-send.r-content-upload-no-session")?;
    if payload.is_empty() {
        return Err("v-send.r-content-upload-empty");
    }
    if payload.len() > MAX_CONTENT_UPLOAD_BYTES {
        return Err("v-send.r-content-upload-too-large");
    }
    let mime_type = parse_content_upload_mime(mime_type)?;
    if let Some(filename) = filename {
        let _ = validate_content_upload_filename(filename)?;
    }
    // matrix-sdk 0.18 `Media::upload` third argument is RequestConfig, not filename.
    let response = client
        .media()
        .upload(&mime_type, payload, None)
        .await
        .map_err(|_| "v-send.r-content-upload-sdk-failed")?;
    let mxc = response.content_uri.to_string();
    if !mxc.starts_with("mxc://") || mxc.as_str().matches('/').count() < 3 {
        return Err("v-send.r-content-upload-sdk-failed");
    }
    Ok(MatrixUploadMediaResult { mxc })
}
