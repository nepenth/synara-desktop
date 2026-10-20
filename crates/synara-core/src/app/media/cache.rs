//! Cached media reads in front of the bounded transport.
//!
//! Two layers, both bounded:
//!
//! - A process-wide in-memory LRU of displayable bytes, keyed by homeserver,
//!   user, the complete media source (including an encrypted file's key, IV and
//!   hash) and the requested format. Revisiting a room or scrolling a
//!   virtualized timeline reuses these bytes without a store read.
//! - The SDK's SQLite media store, which is encrypted with the store
//!   passphrase and governed by `MediaRetentionPolicy` (see
//!   `media_cache::policy`). It holds the bytes exactly as the homeserver
//!   served them, so an encrypted attachment stays ciphertext at rest and is
//!   decrypted with the requesting event's own key. The SDK keys encrypted
//!   sources by URL alone; caching plaintext there would let a second event
//!   that reuses the URL with another key read the first event's plaintext.
//!
//! Ciphertext lives under a Synara-owned synthetic key, never the URL itself:
//! the SDK send queue stores the *plaintext* of our own uploads under the URL
//! key, so sharing it made every attachment we sent to an encrypted room fail
//! its hash check on this device. A stored entry that does not decrypt (an
//! entry from an older build, or another event's key) is refetched once from
//! the homeserver and replaced.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use matrix_sdk::{
    media::{store::IgnoreMediaRetentionPolicy, MediaFormat, MediaRequestParameters, UniqueKey},
    ruma::events::room::MediaSource,
    Client,
};
use sha2::{Digest, Sha256};

use super::bounded::{
    decrypt_attachment_bounded, download_network_bytes_bounded, BoundedMediaError,
};
use crate::app::media_cache::MEDIA_STORE_MAX_FILE_BYTES;

/// Total in-memory budget for hot media bytes.
pub const HOT_MEDIA_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;
/// Larger items (video, big files) are served from the store or the network
/// rather than pinned in memory.
pub const HOT_MEDIA_CACHE_MAX_ITEM_BYTES: usize = 8 * 1024 * 1024;

/// Bytes-bounded least-recently-used map.
///
/// Recency is a monotonically increasing tick per entry; `order` holds
/// `(tick, key)` pairs and stale pairs are skipped during eviction. This keeps
/// every operation amortized O(1) without an intrusive list.
pub struct HotMediaCache {
    max_bytes: usize,
    max_item_bytes: usize,
    total_bytes: usize,
    tick: u64,
    entries: HashMap<String, (u64, Vec<u8>)>,
    order: VecDeque<(u64, String)>,
}

impl HotMediaCache {
    pub fn new(max_bytes: usize, max_item_bytes: usize) -> Self {
        Self {
            max_bytes,
            max_item_bytes: max_item_bytes.min(max_bytes),
            total_bytes: 0,
            tick: 0,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        let entry = self.entries.get_mut(key)?;
        entry.0 = tick;
        let bytes = entry.1.clone();
        self.order.push_back((tick, key.to_owned()));
        self.compact_order();
        Some(bytes)
    }

    pub fn insert(&mut self, key: String, bytes: Vec<u8>) {
        if bytes.is_empty() || bytes.len() > self.max_item_bytes {
            return;
        }
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        if let Some((_, previous)) = self.entries.remove(&key) {
            self.total_bytes -= previous.len();
        }
        self.total_bytes += bytes.len();
        self.entries.insert(key.clone(), (tick, bytes));
        self.order.push_back((tick, key));
        while self.total_bytes > self.max_bytes {
            let Some((tick, key)) = self.order.pop_front() else {
                break;
            };
            if self
                .entries
                .get(&key)
                .is_some_and(|(current, _)| *current == tick)
            {
                if let Some((_, evicted)) = self.entries.remove(&key) {
                    self.total_bytes -= evicted.len();
                }
            }
        }
        self.compact_order();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.total_bytes = 0;
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drop stale recency records once they dominate the queue.
    fn compact_order(&mut self) {
        if self.order.len() <= self.entries.len().saturating_mul(4).max(64) {
            return;
        }
        let entries = &self.entries;
        self.order
            .retain(|(tick, key)| entries.get(key).is_some_and(|(current, _)| current == tick));
    }
}

fn hot_cache() -> &'static Mutex<HotMediaCache> {
    static CACHE: OnceLock<Mutex<HotMediaCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(HotMediaCache::new(
            HOT_MEDIA_CACHE_MAX_BYTES,
            HOT_MEDIA_CACHE_MAX_ITEM_BYTES,
        ))
    })
}

/// Forget every hot media entry. Called when a session closes so decrypted
/// bytes do not outlive the account that could read them.
pub fn clear_hot_media_cache() {
    if let Ok(mut cache) = hot_cache().lock() {
        cache.clear();
    }
}

/// In-memory key: homeserver, user, complete source and format. The encrypted
/// source contributes its key, IV and hashes so plaintext is never shared by
/// two events that merely reuse one URL.
fn hot_key(client: &Client, request: &MediaRequestParameters) -> Option<String> {
    let user_id = client.user_id()?;
    let source = serde_json::to_string(&request.source).ok()?;
    Some(format!(
        "{}\u{1f}{user_id}\u{1f}{source}\u{1f}{}",
        client.homeserver(),
        request.format.unique_key()
    ))
}

/// Synthetic server name for ciphertext store keys. `.invalid` cannot resolve,
/// so no real event can name media there.
const CIPHERTEXT_STORE_SERVER: &str = "synara-ciphertext.invalid";

/// What the homeserver serves for this request, independent of how a
/// particular event decrypts it.
fn network_request(request: &MediaRequestParameters) -> MediaRequestParameters {
    MediaRequestParameters {
        source: MediaSource::Plain(request.uri().to_owned()),
        format: match &request.source {
            // Encrypted media is never thumbnailed by the homeserver.
            MediaSource::Encrypted(_) => MediaFormat::File,
            MediaSource::Plain(_) => request.format.clone(),
        },
    }
}

/// Store key for the network bytes. Plain media shares the SDK's URL key (the
/// SDK stores the same bytes there). Encrypted ciphertext goes under
/// `mxc://synara-ciphertext.invalid/<sha256(url)>`, out of the key space where
/// the SDK keeps plaintext for encrypted sources.
pub(super) fn store_request(request: &MediaRequestParameters) -> MediaRequestParameters {
    let network = network_request(request);
    let MediaSource::Encrypted(_) = &request.source else {
        return network;
    };
    let digest = Sha256::digest(request.uri().as_str().as_bytes());
    let mut uri = format!("mxc://{CIPHERTEXT_STORE_SERVER}/");
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut uri, "{byte:02x}");
    }
    MediaRequestParameters {
        source: MediaSource::Plain(uri.into()),
        format: network.format,
    }
}

fn is_ciphertext_store_uri(request: &MediaRequestParameters) -> bool {
    request
        .uri()
        .parts()
        .is_ok_and(|(server_name, _)| server_name.as_str() == CIPHERTEXT_STORE_SERVER)
}

async fn read_store(client: &Client, request: &MediaRequestParameters) -> Option<Vec<u8>> {
    let store = client.media_store().lock().await.ok()?;
    store.get_media_content(request).await.ok().flatten()
}

async fn write_store(client: &Client, request: &MediaRequestParameters, bytes: Vec<u8>) {
    let Ok(store) = client.media_store().lock().await else {
        return;
    };
    // The retention policy rejects files above its size cap; failures only
    // mean the next read goes to the network again.
    let _ = store
        .add_media_content(request, bytes, IgnoreMediaRetentionPolicy::No)
        .await;
}

fn finish(
    request: &MediaRequestParameters,
    network_bytes: Vec<u8>,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedMediaError> {
    match &request.source {
        MediaSource::Plain(_) => {
            if network_bytes.len() > max_bytes {
                return Err(BoundedMediaError::TooLarge);
            }
            Ok(network_bytes)
        }
        MediaSource::Encrypted(file) => decrypt_attachment_bounded(network_bytes, file, max_bytes),
    }
}

/// Download the network bytes and store them when the policy would keep them.
async fn fetch_and_store(
    client: &Client,
    request: &MediaRequestParameters,
    stored_request: &MediaRequestParameters,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedMediaError> {
    let network_bytes =
        download_network_bytes_bounded(client, &network_request(request), max_bytes).await?;
    // Skip the copy for files the retention policy would refuse anyway.
    if network_bytes.len() as u64 <= MEDIA_STORE_MAX_FILE_BYTES {
        write_store(client, stored_request, network_bytes.clone()).await;
    }
    Ok(network_bytes)
}

async fn remove_store(client: &Client, request: &MediaRequestParameters) {
    if let Ok(store) = client.media_store().lock().await {
        let _ = store.remove_media_content(request).await;
    }
}

/// Bounded media read through the in-memory cache, then the SDK media store,
/// then the network. Every layer enforces `max_bytes`; encrypted sources are
/// decrypted (and their hash checked) with this request's key on every store
/// or network read. A stored entry that fails to decrypt is refetched once.
pub async fn fetch_media_cached(
    client: &Client,
    request: &MediaRequestParameters,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedMediaError> {
    if is_ciphertext_store_uri(request) {
        return Err(BoundedMediaError::InvalidUri);
    }
    let key = hot_key(client, request);
    if let Some(key) = &key {
        let hit = hot_cache().lock().ok().and_then(|mut cache| cache.get(key));
        if let Some(bytes) = hit {
            if bytes.len() > max_bytes {
                return Err(BoundedMediaError::TooLarge);
            }
            return Ok(bytes);
        }
    }

    let stored_request = store_request(request);
    let bytes = match read_store(client, &stored_request).await {
        Some(stored_bytes) => match finish(request, stored_bytes, max_bytes) {
            // The entry is not this event's ciphertext. Trust the homeserver.
            Err(BoundedMediaError::DecryptionFailed) => {
                remove_store(client, &stored_request).await;
                let network_bytes =
                    fetch_and_store(client, request, &stored_request, max_bytes).await?;
                finish(request, network_bytes, max_bytes)?
            }
            result => result?,
        },
        None => {
            let network_bytes =
                fetch_and_store(client, request, &stored_request, max_bytes).await?;
            finish(request, network_bytes, max_bytes)?
        }
    };

    if let Some(key) = key {
        if let Ok(mut cache) = hot_cache().lock() {
            cache.insert(key, bytes.clone());
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_evicts_least_recent_entries_to_the_byte_budget() {
        let mut cache = HotMediaCache::new(10, 10);
        cache.insert("a".into(), vec![1; 4]);
        cache.insert("b".into(), vec![2; 4]);
        assert_eq!(cache.get("a"), Some(vec![1; 4]));
        cache.insert("c".into(), vec![3; 4]);
        assert_eq!(cache.get("b"), None, "b was least recently used");
        assert_eq!(cache.get("a"), Some(vec![1; 4]));
        assert_eq!(cache.get("c"), Some(vec![3; 4]));
        assert_eq!(cache.total_bytes(), 8);
    }

    #[test]
    fn lru_skips_oversized_and_empty_items_and_replaces_in_place() {
        let mut cache = HotMediaCache::new(16, 6);
        cache.insert("big".into(), vec![0; 7]);
        cache.insert("empty".into(), Vec::new());
        assert!(cache.is_empty());
        cache.insert("a".into(), vec![1; 6]);
        cache.insert("a".into(), vec![2; 2]);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.total_bytes(), 2);
        assert_eq!(cache.get("a"), Some(vec![2; 2]));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.total_bytes(), 0);
    }

    #[test]
    fn lru_recency_records_stay_bounded_under_repeated_hits() {
        let mut cache = HotMediaCache::new(64, 8);
        cache.insert("a".into(), vec![1; 1]);
        for _ in 0..10_000 {
            assert!(cache.get("a").is_some());
        }
        assert!(cache.order.len() <= 64);
    }

    #[test]
    fn encrypted_media_is_stored_as_ciphertext_under_its_url() {
        let file: matrix_sdk::ruma::events::room::EncryptedFile =
            serde_json::from_value(serde_json::json!({
                "url": "mxc://example.org/cipher",
                "key": {
                    "kty": "oct",
                    "key_ops": ["encrypt", "decrypt"],
                    "alg": "A256CTR",
                    "k": "qcHVMSgYg-71CauWBezXI5qkaRb0LuIy-Wx5kIaHMIA",
                    "ext": true
                },
                "iv": "X85+XgHN+HEAAAAAAAAAAA",
                "hashes": { "sha256": "5qG4fFnbbVdlAB1Q72JDKwCagV6Dbkx9uOaadRkQnMs" },
                "v": "v2"
            }))
            .expect("encrypted file fixture");
        let request = MediaRequestParameters {
            source: MediaSource::Encrypted(Box::new(file)),
            format: MediaFormat::Thumbnail(matrix_sdk::media::MediaThumbnailSettings::new(
                96_u32.into(),
                96_u32.into(),
            )),
        };
        // The homeserver is asked for the file at its URL...
        let network = network_request(&request);
        assert!(
            matches!(network.source, MediaSource::Plain(ref uri) if uri.as_str() == "mxc://example.org/cipher")
        );
        assert!(matches!(network.format, MediaFormat::File));

        // ...but the ciphertext is stored outside the SDK's URL key space.
        let stored = store_request(&request);
        let MediaSource::Plain(uri) = &stored.source else {
            panic!("ciphertext is stored under a plain synthetic key");
        };
        let (server_name, media_id) = uri.parts().expect("valid synthetic mxc uri");
        assert_eq!(server_name.as_str(), CIPHERTEXT_STORE_SERVER);
        assert_eq!(media_id.len(), 64);
        assert!(media_id.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(matches!(stored.format, MediaFormat::File));
        assert_ne!(
            stored.unique_key(),
            MediaRequestParameters {
                source: request.source.clone(),
                format: MediaFormat::File,
            }
            .unique_key()
        );
        assert_eq!(stored.unique_key(), store_request(&request).unique_key());
        assert!(is_ciphertext_store_uri(&stored));
        assert!(!is_ciphertext_store_uri(&request));
    }

    #[test]
    fn plain_media_keeps_the_sdk_url_key() {
        let request = MediaRequestParameters {
            source: MediaSource::Plain("mxc://example.org/plain".into()),
            format: MediaFormat::File,
        };
        assert_eq!(store_request(&request).unique_key(), request.unique_key());
    }

    /// Regression for the send queue's own media cache write: after an upload
    /// to an encrypted room the SDK stores the *plaintext* under the encrypted
    /// source's URL key, which Synara used to read as ciphertext.
    #[tokio::test]
    async fn own_encrypted_attachment_sent_through_the_send_queue_reads_back() {
        use std::sync::{Arc, Mutex as StdMutex};

        use matrix_sdk::{
            attachment::AttachmentConfig, send_queue::RoomSendQueueUpdate,
            test_utils::mocks::MatrixMockServer,
        };
        use matrix_sdk_test::DEFAULT_TEST_ROOM_ID;
        use wiremock::{Request, ResponseTemplate};

        let plaintext = b"own-encrypted-photo".to_vec();
        let mock = MatrixMockServer::new().await;
        mock.mock_authenticated_media_config()
            .ok_default()
            .mount()
            .await;
        let uploaded = Arc::new(StdMutex::new(Vec::<u8>::new()));
        let uploaded_by_server = Arc::clone(&uploaded);
        mock.mock_upload()
            .respond_with(move |request: &Request| {
                *uploaded_by_server.lock().unwrap() = request.body.clone();
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "content_uri": "mxc://example.org/sent" }))
            })
            .mock_once()
            .mount()
            .await;
        mock.mock_get_members().ok(Vec::new()).mount().await;
        mock.mock_room_send()
            .ok(matrix_sdk::ruma::event_id!("$sent-photo"))
            .mock_once()
            .mount()
            .await;
        let served = Arc::clone(&uploaded);
        mock.mock_authed_media_download()
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(200)
                    .set_body_raw(served.lock().unwrap().clone(), "application/octet-stream")
            })
            .mock_once()
            .mount()
            .await;

        let client = mock.client_builder().build().await;
        let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
        mock.mock_room_state_encryption().encrypted().mount().await;
        let (_, mut updates) = room.send_queue().subscribe().await.expect("subscribe");

        crate::app::send::send_attachment_via_room_queue(
            &room,
            "photo.png",
            mime::IMAGE_PNG,
            plaintext.clone(),
            AttachmentConfig::new(),
        )
        .await
        .expect("send attachment through RoomSendQueue");

        let mut sent_source = None;
        while let Ok(update) = updates.try_recv() {
            if let RoomSendQueueUpdate::MediaUpload {
                file: Some(source), ..
            } = update
            {
                sent_source = Some(source);
            }
        }
        let source = sent_source.expect("uploaded media source");
        let MediaSource::Encrypted(file) = &source else {
            panic!("an encrypted room uploads an encrypted file");
        };
        assert_eq!(file.url.as_str(), "mxc://example.org/sent");
        assert_ne!(
            *uploaded.lock().unwrap(),
            plaintext,
            "the upload is ciphertext"
        );

        let request = MediaRequestParameters {
            source: source.clone(),
            format: MediaFormat::File,
        };
        // The SDK's own write: plaintext under the encrypted source's URL key.
        assert_eq!(
            read_store(&client, &request).await.as_deref(),
            Some(plaintext.as_slice())
        );

        // One network read of the ciphertext, then the store.
        for _ in 0..2 {
            assert_eq!(
                fetch_media_cached(&client, &request, 1024)
                    .await
                    .expect("own encrypted attachment"),
                plaintext
            );
        }
        clear_hot_media_cache_for_test(&client, &request);
        assert_eq!(
            fetch_media_cached(&client, &request, 1024)
                .await
                .expect("own encrypted attachment from the store"),
            plaintext
        );
        assert_eq!(
            read_store(&client, &store_request(&request)).await,
            Some(uploaded.lock().unwrap().clone()),
            "the store holds the served ciphertext under Synara's key"
        );
    }

    fn clear_hot_media_cache_for_test(client: &Client, request: &MediaRequestParameters) {
        let key = hot_key(client, request).expect("hot key");
        let mut cache = hot_cache().lock().unwrap();
        if let Some((_, bytes)) = cache.entries.remove(&key) {
            cache.total_bytes -= bytes.len();
        }
    }
}
