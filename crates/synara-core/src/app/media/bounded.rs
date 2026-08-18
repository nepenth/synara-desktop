//! Bounded Matrix media download owner.
//!
//! The SDK's high-level media helper materializes the complete response before
//! callers can inspect its length. This path reuses the SDK's configured HTTP
//! client and native access token, but rejects oversized Content-Length values
//! and stops chunked responses as soon as the product cap is crossed.

use std::io::{Cursor, Read};

use futures_util::StreamExt;
use matrix_sdk::{
    media::{MediaFormat, MediaRequestParameters},
    ruma::{events::room::MediaSource, MxcUri},
    Client,
};
use reqwest::{Response, StatusCode, Url};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundedMediaError {
    InvalidUri,
    MissingSession,
    RequestFailed,
    TooLarge,
    DecryptionFailed,
}

fn endpoint_url(
    client: &Client,
    uri: &MxcUri,
    format: &MediaFormat,
    authenticated_endpoint: bool,
) -> Result<Url, BoundedMediaError> {
    let (server_name, media_id) = uri.parts().map_err(|_| BoundedMediaError::InvalidUri)?;
    let mut url = client.homeserver();
    url.set_query(None);
    url.set_fragment(None);

    let mut base_segments = url
        .path_segments()
        .ok_or(BoundedMediaError::InvalidUri)?
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if authenticated_endpoint {
        base_segments.extend(["_matrix", "client", "v1", "media"].map(str::to_owned));
    } else {
        base_segments.extend(["_matrix", "media", "v3"].map(str::to_owned));
    }
    base_segments.push(
        match format {
            MediaFormat::File => "download",
            MediaFormat::Thumbnail(_) => "thumbnail",
        }
        .to_owned(),
    );
    base_segments.push(server_name.as_str().to_owned());
    base_segments.push(media_id.to_owned());

    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| BoundedMediaError::InvalidUri)?;
        segments.clear();
        segments.extend(base_segments);
    }

    if let MediaFormat::Thumbnail(settings) = format {
        url.query_pairs_mut()
            .append_pair("width", &settings.width.to_string())
            .append_pair("height", &settings.height.to_string())
            .append_pair("method", settings.method.as_ref())
            .append_pair("animated", if settings.animated { "true" } else { "false" });
    }
    Ok(url)
}

async fn request_media(
    client: &Client,
    uri: &MxcUri,
    format: &MediaFormat,
    authenticated_endpoint: bool,
) -> Result<Response, BoundedMediaError> {
    let url = endpoint_url(client, uri, format, authenticated_endpoint)?;
    let access_token = client
        .access_token()
        .ok_or(BoundedMediaError::MissingSession)?;
    client
        .http_client()
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| BoundedMediaError::RequestFailed)
}

fn extend_bounded(
    destination: &mut Vec<u8>,
    chunk: &[u8],
    max_bytes: usize,
) -> Result<(), BoundedMediaError> {
    if destination
        .len()
        .checked_add(chunk.len())
        .is_none_or(|size| size > max_bytes)
    {
        return Err(BoundedMediaError::TooLarge);
    }
    destination.extend_from_slice(chunk);
    Ok(())
}

async fn read_response_bounded(
    response: Response,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedMediaError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(BoundedMediaError::TooLarge);
    }
    if !response.status().is_success() {
        return Err(BoundedMediaError::RequestFailed);
    }

    let mut bytes = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or_default()
            .min(max_bytes as u64) as usize,
    );
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| BoundedMediaError::RequestFailed)?;
        extend_bounded(&mut bytes, &chunk, max_bytes)?;
    }
    Ok(bytes)
}

/// Download and, when needed, decrypt a Matrix media response without ever
/// buffering more than `max_bytes` of attacker-controlled network content.
pub async fn download_media_bounded(
    client: &Client,
    request: &MediaRequestParameters,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedMediaError> {
    let (uri, encrypted_file) = match &request.source {
        MediaSource::Plain(uri) => (uri.as_ref(), None),
        MediaSource::Encrypted(file) => (file.url.as_ref(), Some(file.as_ref())),
    };

    let mut response = request_media(client, uri, &request.format, true).await?;
    if matches!(
        response.status(),
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ) {
        response = request_media(client, uri, &request.format, false).await?;
    }
    let ciphertext = read_response_bounded(response, max_bytes).await?;

    let Some(file) = encrypted_file else {
        return Ok(ciphertext);
    };
    let mut cursor = Cursor::new(ciphertext);
    let mut decryptor =
        matrix_sdk_crypto::AttachmentDecryptor::new(&mut cursor, file.clone().into())
            .map_err(|_| BoundedMediaError::DecryptionFailed)?;
    let mut plaintext = Vec::new();
    decryptor
        .by_ref()
        .take(max_bytes.saturating_add(1) as u64)
        .read_to_end(&mut plaintext)
        .map_err(|_| BoundedMediaError::DecryptionFailed)?;
    if plaintext.len() > max_bytes {
        return Err(BoundedMediaError::TooLarge);
    }
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_accumulator_rejects_the_chunk_that_crosses_the_cap() {
        let mut bytes = vec![1, 2];
        assert_eq!(extend_bounded(&mut bytes, &[3, 4], 4), Ok(()));
        assert_eq!(
            extend_bounded(&mut bytes, &[5], 4),
            Err(BoundedMediaError::TooLarge)
        );
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }
}
