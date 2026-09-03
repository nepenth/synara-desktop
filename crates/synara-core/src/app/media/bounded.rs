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

fn decrypt_attachment_bounded(
    ciphertext: Vec<u8>,
    file: &matrix_sdk::ruma::events::room::EncryptedFile,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedMediaError> {
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
    decrypt_attachment_bounded(ciphertext, file, max_bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::time::{Duration, Instant};

    use matrix_sdk::{
        authentication::matrix::MatrixSession,
        ruma::{
            events::room::{EncryptedFile, MediaSource},
            OwnedDeviceId, OwnedMxcUri, UserId,
        },
        store::RoomLoadSettings,
        Client, SessionMeta, SessionTokens,
    };
    use matrix_sdk_crypto::AttachmentEncryptor;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::oneshot,
    };

    use super::*;

    const LOOPBACK_TEST_TIMEOUT: Duration = Duration::from_secs(10);

    async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 2_048];
        loop {
            let count = socket.read(&mut chunk).await.expect("read request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8(request).expect("request is ASCII HTTP")
    }

    async fn serve_responses(listener: TcpListener, responses: Vec<String>) -> Vec<String> {
        let mut requests = Vec::with_capacity(responses.len());
        let mut responses = VecDeque::from(responses);
        while let Some(next_response) = responses.pop_front() {
            let (mut socket, _) = listener.accept().await.expect("accept media request");
            let request = read_request(&mut socket).await;
            if !request
                .lines()
                .next()
                .is_some_and(|line| line.contains("/media/"))
            {
                socket
                    .write_all(response("404 Not Found", b"").as_bytes())
                    .await
                    .expect("write auxiliary response");
                socket.shutdown().await.expect("close auxiliary response");
                responses.push_front(next_response);
                continue;
            }
            requests.push(request);
            socket
                .write_all(next_response.as_bytes())
                .await
                .expect("write media response");
            socket.shutdown().await.expect("close media response");
        }
        requests
    }

    async fn restored_client(listener: &TcpListener) -> Client {
        let client = Client::builder()
            .homeserver_url(format!(
                "http://{}",
                listener.local_addr().expect("loopback address")
            ))
            .build()
            .await
            .expect("build loopback client");
        client
            .matrix_auth()
            .restore_session(
                MatrixSession {
                    meta: SessionMeta {
                        user_id: UserId::parse("@alice:example.org").expect("user id"),
                        device_id: OwnedDeviceId::from("MEDIADEVICE"),
                    },
                    tokens: SessionTokens {
                        access_token: "media-proof-token".to_owned(),
                        refresh_token: None,
                    },
                },
                RoomLoadSettings::default(),
            )
            .await
            .expect("restore loopback media session");
        client
    }

    fn media_request() -> MediaRequestParameters {
        MediaRequestParameters {
            source: MediaSource::Plain(OwnedMxcUri::from("mxc://example.org/proof-media")),
            format: MediaFormat::File,
        }
    }

    fn response(status: &str, body: &[u8]) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            String::from_utf8_lossy(body)
        )
    }

    fn encrypted_attachment(plaintext: &[u8]) -> (Vec<u8>, EncryptedFile) {
        let mut cursor = Cursor::new(plaintext);
        let mut encryptor = AttachmentEncryptor::new(&mut cursor);
        let mut ciphertext = Vec::new();
        encryptor
            .read_to_end(&mut ciphertext)
            .expect("encrypt attachment fixture");
        let info = encryptor.finish();
        let file = EncryptedFile::new(
            OwnedMxcUri::from("mxc://example.org/encrypted-proof-media"),
            info.encryption_info,
            info.hashes,
        );
        (ciphertext, file)
    }

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

    #[test]
    fn encrypted_attachment_decrypts_in_cap_and_rejects_plaintext_overflow() {
        let expected = b"encrypted-media";
        let (ciphertext, file) = encrypted_attachment(expected);
        assert_eq!(
            decrypt_attachment_bounded(ciphertext, &file, expected.len()),
            Ok(expected.to_vec())
        );

        let oversized = vec![0x5a; 65];
        let (ciphertext, file) = encrypted_attachment(&oversized);
        assert_eq!(
            decrypt_attachment_bounded(ciphertext, &file, 64),
            Err(BoundedMediaError::TooLarge)
        );
    }

    #[tokio::test]
    async fn loopback_fetch_measures_real_sdk_boundary_and_authenticated_route() {
        tokio::time::timeout(LOOPBACK_TEST_TIMEOUT, async {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind loopback media server");
            let client = restored_client(&listener).await;
            let server = tokio::spawn(serve_responses(
                listener,
                vec![response("200 OK", b"measured-media")],
            ));

            let started = Instant::now();
            let bytes = download_media_bounded(&client, &media_request(), 64)
                .await
                .expect("bounded media fetch");
            let elapsed = started.elapsed();
            let requests = server.await.expect("media server");

            assert_eq!(bytes, b"measured-media");
            assert!(elapsed < Duration::from_secs(5));
            assert_eq!(requests.len(), 1);
            assert!(requests[0].starts_with(
                "GET /_matrix/client/v1/media/download/example.org/proof-media HTTP/1.1"
            ));
            assert!(requests[0].contains("authorization: Bearer media-proof-token"));
        })
        .await
        .expect("authenticated loopback proof timed out");
    }

    #[tokio::test]
    async fn content_length_and_chunked_responses_fail_closed_at_the_exact_cap() {
        tokio::time::timeout(LOOPBACK_TEST_TIMEOUT, async {
        let content_length_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind content-length server");
        let content_length_client = restored_client(&content_length_listener).await;
        let content_length_server = tokio::spawn(serve_responses(
            content_length_listener,
            vec!["HTTP/1.1 200 OK\r\nContent-Length: 65\r\nConnection: close\r\n\r\n".to_owned()],
        ));
        assert_eq!(
            download_media_bounded(&content_length_client, &media_request(), 64).await,
            Err(BoundedMediaError::TooLarge)
        );
        content_length_server.await.expect("content-length server");

        let chunked_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind chunked server");
        let chunked_client = restored_client(&chunked_listener).await;
        let chunked_server = tokio::spawn(serve_responses(
            chunked_listener,
            vec![
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n4\r\n1234\r\n4\r\n5678\r\n0\r\n\r\n"
                    .to_owned(),
            ],
        ));
        assert_eq!(
            download_media_bounded(&chunked_client, &media_request(), 6).await,
            Err(BoundedMediaError::TooLarge)
        );
        chunked_server.await.expect("chunked server");
        })
        .await
        .expect("bounded-response loopback proof timed out");
    }

    #[tokio::test]
    async fn retry_measurement_is_only_the_documented_legacy_endpoint_fallback() {
        tokio::time::timeout(LOOPBACK_TEST_TIMEOUT, async {
            for fallback_status in ["404 Not Found", "405 Method Not Allowed"] {
                let listener = TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("bind fallback server");
                let client = restored_client(&listener).await;
                let server = tokio::spawn(serve_responses(
                    listener,
                    vec![
                        response(fallback_status, b""),
                        response("200 OK", b"legacy-media"),
                    ],
                ));

                let bytes = download_media_bounded(&client, &media_request(), 64)
                    .await
                    .expect("legacy fallback fetch");
                let requests = server.await.expect("fallback server");
                assert_eq!(bytes, b"legacy-media");
                assert_eq!(requests.len(), 2);
                assert!(requests[0].contains("/_matrix/client/v1/media/download/"));
                assert!(requests[1].contains("/_matrix/media/v3/download/"));
            }

            let failure_listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind failure server");
            let failure_client = restored_client(&failure_listener).await;
            let failure_server = tokio::spawn(serve_responses(
                failure_listener,
                vec![response("500 Internal Server Error", b"")],
            ));
            assert_eq!(
                download_media_bounded(&failure_client, &media_request(), 64).await,
                Err(BoundedMediaError::RequestFailed)
            );
            assert_eq!(failure_server.await.expect("failure server").len(), 1);
        })
        .await
        .expect("fallback loopback proof timed out");
    }

    #[tokio::test]
    async fn bounded_transport_refetches_and_calling_task_can_be_cancelled() {
        tokio::time::timeout(LOOPBACK_TEST_TIMEOUT, async {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind repetition server");
            let client = restored_client(&listener).await;
            let server = tokio::spawn(serve_responses(
                listener,
                vec![response("200 OK", b"same"), response("200 OK", b"same")],
            ));
            for _ in 0..2 {
                assert_eq!(
                    download_media_bounded(&client, &media_request(), 64)
                        .await
                        .expect("repeated fetch"),
                    b"same"
                );
            }
            assert_eq!(server.await.expect("repetition server").len(), 2);

            let cancellation_listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind cancellation server");
            let cancellation_client = restored_client(&cancellation_listener).await;
            let (first_chunk_sent, first_chunk_received) = oneshot::channel();
            let cancellation_server = tokio::spawn(async move {
                let (mut socket, _) = cancellation_listener
                    .accept()
                    .await
                    .expect("accept cancellation request");
                let _ = read_request(&mut socket).await;
                socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n4\r\nwait\r\n",
                    )
                    .await
                    .expect("write first chunk");
                let _ = first_chunk_sent.send(());
                std::future::pending::<()>().await;
            });
            let fetch = tokio::spawn(async move {
                download_media_bounded(&cancellation_client, &media_request(), 64).await
            });
            first_chunk_received.await.expect("first media chunk");
            fetch.abort();
            assert!(fetch
                .await
                .expect_err("fetch must be cancelled")
                .is_cancelled());
            cancellation_server.abort();
            assert!(cancellation_server
                .await
                .expect_err("fixture server must be cancelled")
                .is_cancelled());
        })
        .await
        .expect("refetch and caller-cancellation loopback proof timed out");
    }
}
