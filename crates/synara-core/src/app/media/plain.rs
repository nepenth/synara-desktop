//! Live plain `mxc://` download. Bytes stay method returns, never Core JSON.
//!
//! Timeline-media handles stay on `timeline_media_bytes`. Encrypted
//! `MediaSource` is not this API. The byte cap matches desktop
//! `MAX_MEDIA_DOWNLOAD_BYTES` (300 MiB).

use matrix_sdk::{
    media::{MediaFormat, MediaRequestParameters, MediaThumbnailSettings},
    ruma::{events::room::MediaSource, OwnedMxcUri, UInt},
    Client,
};

use super::{download_media_bounded, BoundedMediaError};

/// Same URI bound as desktop `parse_media_download_uri`.
pub const MAX_PLAIN_MEDIA_URI_BYTES: usize = 2048;
/// Same original-file ceiling as desktop `MAX_MEDIA_DOWNLOAD_BYTES`.
pub const MAX_PLAIN_MEDIA_DOWNLOAD_BYTES: usize = 300 * 1024 * 1024;

const INVALID_URI: &str = "v-send.r-media-invalid-content-uri";
const TOO_LARGE: &str = "v-send.r-media-download-too-large";
const SDK_FAILED: &str = "v-send.r-media-download-sdk-failed";
const TIMELINE_MEDIA_PREFIX: &str = "timeline-media-";

pub fn parse_plain_media_uri(content_uri: &str) -> Result<OwnedMxcUri, &'static str> {
    if content_uri.starts_with(TIMELINE_MEDIA_PREFIX) {
        return Err(INVALID_URI);
    }
    if content_uri.is_empty()
        || content_uri.len() > MAX_PLAIN_MEDIA_URI_BYTES
        || content_uri != content_uri.trim()
        || !content_uri.is_ascii()
        || content_uri.contains(['?', '#'])
    {
        return Err(INVALID_URI);
    }

    let owned = OwnedMxcUri::from(content_uri);
    let valid = owned.validate().is_ok()
        && owned
            .media_id()
            .map(|media_id| !media_id.is_empty())
            .unwrap_or(false);
    if !valid {
        return Err(INVALID_URI);
    }
    Ok(owned)
}

pub fn parse_plain_media_thumbnail_size(
    width: u64,
    height: u64,
) -> Result<(UInt, UInt), &'static str> {
    if width == 0 || height == 0 {
        return Err(INVALID_URI);
    }
    let width = UInt::new(width).ok_or(INVALID_URI)?;
    let height = UInt::new(height).ok_or(INVALID_URI)?;
    Ok((width, height))
}

fn map_bounded_error(error: BoundedMediaError) -> &'static str {
    match error {
        BoundedMediaError::TooLarge => TOO_LARGE,
        BoundedMediaError::InvalidUri => INVALID_URI,
        _ => SDK_FAILED,
    }
}

/// Original-file download for a validated plain `mxc://`.
pub async fn download_plain_media(
    client: &Client,
    content_uri: &str,
) -> Result<Vec<u8>, &'static str> {
    let uri = parse_plain_media_uri(content_uri)?;
    let request = MediaRequestParameters {
        source: MediaSource::Plain(uri),
        format: MediaFormat::File,
    };
    download_media_bounded(client, &request, MAX_PLAIN_MEDIA_DOWNLOAD_BYTES)
        .await
        .map_err(map_bounded_error)
}

/// Thumbnail download for a validated plain `mxc://`.
pub async fn thumbnail_plain_media(
    client: &Client,
    content_uri: &str,
    width: u64,
    height: u64,
) -> Result<Vec<u8>, &'static str> {
    let uri = parse_plain_media_uri(content_uri)?;
    let (width, height) = parse_plain_media_thumbnail_size(width, height)?;
    let request = MediaRequestParameters {
        source: MediaSource::Plain(uri),
        format: MediaFormat::Thumbnail(MediaThumbnailSettings::new(width, height)),
    };
    download_media_bounded(client, &request, MAX_PLAIN_MEDIA_DOWNLOAD_BYTES)
        .await
        .map_err(map_bounded_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_media_uri_matches_desktop_guards() {
        assert_eq!(
            parse_plain_media_uri("mxc://example.org/media")
                .unwrap()
                .to_string(),
            "mxc://example.org/media"
        );

        for invalid in [
            "",
            " ",
            "https://example.org/media",
            "data:text/plain,secret",
            "javascript:alert(1)",
            "mxc://example.org/",
            "mxc://example.org/media?access_token=secret",
            "mxc://example.org/media#fragment",
            "mxc://example.org/me/dia",
            " mxc://example.org/media",
            "timeline-media-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &format!("mxc://example.org/{}", "a".repeat(2_100)),
        ] {
            assert_eq!(
                parse_plain_media_uri(invalid),
                Err(INVALID_URI),
                "invalid URI should be rejected: {invalid}"
            );
        }
    }

    #[test]
    fn parse_plain_media_thumbnail_size_rejects_zero_and_overflow() {
        assert!(parse_plain_media_thumbnail_size(96, 96).is_ok());
        assert_eq!(parse_plain_media_thumbnail_size(0, 96), Err(INVALID_URI));
        assert_eq!(parse_plain_media_thumbnail_size(96, 0), Err(INVALID_URI));
        assert_eq!(
            parse_plain_media_thumbnail_size(u64::MAX, 96),
            Err(INVALID_URI)
        );
    }
}
