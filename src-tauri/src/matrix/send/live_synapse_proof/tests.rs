//! Opt-in live V-SEND.1 proof against disposable Synapse.
//!
//! Path marker `.../tests.rs` keeps this file outside production matrix
//! client/sync guardrails while remaining crate-private.
//!
//! Gated by:
//! - `SYNARA_RUN_MATRIX_RUST_ATTACHMENT_LIVE=1`
//! - `SYNARA_MATRIX_HOMESERVER_URL=http://127.0.0.1:<port>` (credential-free HTTP loopback)
//!
//! Exercises the native composer attachment owner end-to-end:
//! register/login → create room → `AttachmentSendQueue` enqueue →
//! `Room::send_attachment` (same SDK path as `matrix_send_attachment`) →
//! mark sent → native timeline readback of the media event.
//!
//! JS two-client Synapse CI is not this proof. WebView click-through is not required.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
use mime::Mime;
use sha1::Sha1;

use crate::matrix::auth::{login_with_password, LoginOptions};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::send::{AttachmentEnqueue, AttachmentKind, AttachmentSendQueue};
use crate::matrix::store::{AccountIdentity, StoreKeyMaterial};
use crate::matrix::timeline::{NativeTimelineDirection, NativeTimelineRegistry};

type HmacSha1 = Hmac<Sha1>;

/// Minimal valid 1×1 PNG (68 bytes).
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xfe, 0xd4, 0xef, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_ATTACHMENT_LIVE")
        .ok()
        .as_deref()
        == Some("1")
}

fn loopback_homeserver_url() -> Option<String> {
    let raw = std::env::var("SYNARA_MATRIX_HOMESERVER_URL").ok()?;
    let Ok(url) = url::Url::parse(&raw) else {
        return None;
    };
    if url.scheme() != "http"
        || !matches!(
            url.host_str(),
            Some("127.0.0.1") | Some("localhost") | Some("::1")
        )
        || !url.username().is_empty()
        || url.password().is_some()
        || !(url.path().is_empty() || url.path() == "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

fn registration_secret_from_harness() -> String {
    let config = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("integration/synapse/runtime/homeserver.yaml");
    let text = std::fs::read_to_string(&config)
        .unwrap_or_else(|err| panic!("read disposable Synapse config: {err}"));
    let mut secret = None;
    for line in text.lines() {
        let Some(value) = line.strip_prefix("registration_shared_secret:") else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        assert!(
            value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()),
            "disposable Synapse registration secret must be one generated 64-hex value"
        );
        assert!(secret.is_none(), "duplicate registration_shared_secret");
        secret = Some(value.to_owned());
    }
    secret.expect("disposable Synapse registration secret")
}

fn registration_mac(secret: &str, nonce: &str, localpart: &str, password: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(nonce.as_bytes());
    mac.update(&[0]);
    mac.update(localpart.as_bytes());
    mac.update(&[0]);
    mac.update(password.as_bytes());
    mac.update(&[0]);
    mac.update(b"notadmin");
    hex::encode(mac.finalize().into_bytes())
}

async fn register_disposable_account(
    base_url: &str,
    localpart: &str,
    password: &str,
) -> serde_json::Value {
    let secret = registration_secret_from_harness();
    let endpoint = format!("{base_url}/_synapse/admin/v1/register");
    let http = reqwest::Client::builder().build().expect("http client");
    let nonce_response = http
        .get(&endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .expect("registration nonce request");
    assert!(
        nonce_response.status().is_success(),
        "registration nonce HTTP {}",
        nonce_response.status()
    );
    let nonce_text = nonce_response.text().await.expect("nonce body");
    let nonce_json: serde_json::Value = serde_json::from_str(&nonce_text).expect("nonce json");
    let nonce = nonce_json["nonce"]
        .as_str()
        .expect("registration nonce")
        .to_owned();
    let mac = registration_mac(&secret, &nonce, localpart, password);
    let body = serde_json::json!({
        "nonce": nonce,
        "username": localpart,
        "password": password,
        "admin": false,
        "mac": mac,
    })
    .to_string();
    let register_response = http
        .post(&endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .expect("shared-secret registration");
    assert!(
        register_response.status().is_success(),
        "registration HTTP {}",
        register_response.status()
    );
    let register_text = register_response.text().await.expect("registration body");
    serde_json::from_str(&register_text).expect("registration json")
}

fn temp_store_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-v-send1-attachment-{nanos}"));
    std::fs::create_dir_all(&root).expect("temp store root");
    root
}

async fn sync_briefly(client: &matrix_sdk::Client) {
    let _ = client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(1)))
        .await;
}

async fn wait_for_event_in_open_timeline(
    registry: &mut NativeTimelineRegistry,
    client: &matrix_sdk::Client,
    room_id: &str,
    event_id: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        sync_briefly(client).await;
        let _ = registry
            .paginate_legacy(client, room_id, NativeTimelineDirection::Backwards)
            .await;
        let snapshot = registry
            .snapshot(client, room_id)
            .await
            .expect("timeline snapshot");
        if snapshot.items.iter().any(|item| item.event_id == event_id) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for attachment event {event_id} in open native timeline (items={})",
            snapshot.items.len()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[tokio::test]
async fn live_native_attachment_send_against_disposable_synapse_when_configured() {
    if !live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let localpart = format!("vs1_{stamp:x}");
    let password = format!("S-{stamp:x}");
    let registration = register_disposable_account(&base, &localpart, &password).await;
    let user_id = registration["user_id"]
        .as_str()
        .expect("registration user_id")
        .to_owned();

    let store_root = temp_store_root();
    let identity = AccountIdentity::new(&user_id, &base).expect("account identity");
    let key = StoreKeyMaterial::generate().expect("store key");
    let config = ClientBuildConfig::product_default(&store_root, identity, Some(key))
        .expect("client config");
    let client = build_unauthenticated_client(&config)
        .await
        .expect("unauthenticated client");
    login_with_password(
        &client,
        &localpart,
        &password,
        &LoginOptions {
            device_display_name: Some("V-SEND.1 attachment proof".into()),
            request_refresh_token: false,
            device_id: None,
        },
    )
    .await
    .expect("password login");

    client
        .event_cache()
        .subscribe()
        .expect("subscribe event cache for native attachment proof");

    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("initial sync");

    let room = client
        .create_room(CreateRoomRequest::new())
        .await
        .expect("create room");
    let room_id = room.room_id().to_string();
    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("post-create sync");

    let mut registry = NativeTimelineRegistry::new(1);
    registry
        .open(&client, &room_id)
        .await
        .expect("open native timeline before attachment send");

    let filename = "v-send.1-proof.png";
    let mime_type: Mime = "image/png".parse().expect("image/png mime");
    let mut queue = AttachmentSendQueue::new(1);
    let enqueued = queue
        .enqueue(AttachmentEnqueue {
            room_id: room_id.clone(),
            kind: AttachmentKind::Image,
            media_handle_id: format!("native-staged:{filename}"),
            file_name: Some(filename.to_owned()),
            caption: None,
            mime_type: Some(mime_type.to_string()),
            size_bytes: Some(TINY_PNG.len() as u64),
        })
        .expect("enqueue native attachment");
    let local_txn_id = enqueued.local_txn_id.clone();

    // Same SDK owner path as `matrix_send_attachment` in product.rs.
    let response = room
        .send_attachment(
            filename,
            &mime_type,
            TINY_PNG.to_vec(),
            AttachmentConfig::new(),
        )
        .await
        .expect("send_attachment via managed client");
    let event_id = response.event_id.to_string();
    queue
        .mark_sent(&local_txn_id)
        .expect("mark attachment sent");

    wait_for_event_in_open_timeline(&mut registry, &client, &room_id, &event_id).await;

    let snapshot = registry
        .snapshot(&client, &room_id)
        .await
        .expect("post-send snapshot");
    let item = snapshot
        .items
        .iter()
        .find(|item| item.event_id == event_id)
        .expect("attachment event in native timeline");
    assert!(
        item.event_type.contains("m.room.message") || item.event_type.contains("image"),
        "expected media message type, got {}",
        item.event_type
    );

    // Second path: ordinary file attachment (not image MIME).
    let file_name = "v-send.1-proof.txt";
    let file_mime: Mime = "text/plain".parse().expect("text/plain mime");
    let file_bytes = b"v-send.1 attachment proof file\n".to_vec();
    let file_enqueued = queue
        .enqueue(AttachmentEnqueue {
            room_id: room_id.clone(),
            kind: AttachmentKind::File,
            media_handle_id: format!("native-staged:{file_name}"),
            file_name: Some(file_name.to_owned()),
            caption: None,
            mime_type: Some(file_mime.to_string()),
            size_bytes: Some(file_bytes.len() as u64),
        })
        .expect("enqueue file attachment");
    let file_txn = file_enqueued.local_txn_id.clone();
    let file_response = room
        .send_attachment(file_name, &file_mime, file_bytes, AttachmentConfig::new())
        .await
        .expect("send file attachment");
    let file_event_id = file_response.event_id.to_string();
    queue.mark_sent(&file_txn).expect("mark file sent");
    wait_for_event_in_open_timeline(&mut registry, &client, &room_id, &file_event_id).await;

    let _ = std::fs::remove_dir_all(&store_root);
}

fn poll_live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_POLL_LIVE")
        .ok()
        .as_deref()
        == Some("1")
}

#[tokio::test]
async fn live_native_poll_send_and_respond_against_disposable_synapse_when_configured() {
    if !poll_live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let localpart = format!("vs3_{stamp:x}");
    let password = format!("S-{stamp:x}");
    let registration = register_disposable_account(&base, &localpart, &password).await;
    let user_id = registration["user_id"]
        .as_str()
        .expect("registration user_id")
        .to_owned();

    let store_root = temp_store_root();
    let identity = AccountIdentity::new(&user_id, &base).expect("account identity");
    let key = StoreKeyMaterial::generate().expect("store key");
    let config = ClientBuildConfig::product_default(&store_root, identity, Some(key))
        .expect("client config");
    let client = build_unauthenticated_client(&config)
        .await
        .expect("unauthenticated client");
    login_with_password(
        &client,
        &localpart,
        &password,
        &LoginOptions {
            device_display_name: Some("V-SEND.3 poll proof".into()),
            request_refresh_token: false,
            device_id: None,
        },
    )
    .await
    .expect("password login");

    client
        .event_cache()
        .subscribe()
        .expect("subscribe event cache for native poll proof");

    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("initial sync");

    let room = client
        .create_room(CreateRoomRequest::new())
        .await
        .expect("create room");
    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("post-create sync");

    // Send-path proof: native timeline DTO projection of poll kinds is a later
    // V-TIMELINE residual. Authoritative V-SEND.3 evidence is Room::send plus
    // managed-client fetch of the resulting events from Synapse.
    let normalized = crate::matrix::send::normalize_poll(
        "V-SEND.3 proof?",
        &["Alpha".to_owned(), "Beta".to_owned()],
        1,
    )
    .expect("normalize poll");
    let start = crate::matrix::send::poll_start_content(&normalized).expect("poll start content");
    let start_response = room
        .send(start)
        .await
        .expect("send poll start via managed client");
    let poll_event_id = start_response.response.event_id.clone();
    wait_for_room_event(&client, &room, &poll_event_id).await;

    let answer_id = normalized.answers[0].0.clone();
    let response_content =
        crate::matrix::send::poll_response_content(poll_event_id.as_str(), &[answer_id])
            .expect("poll response content");
    let vote_response = room
        .send(response_content)
        .await
        .expect("send poll response via managed client");
    let vote_event_id = vote_response.response.event_id.clone();
    wait_for_room_event(&client, &room, &vote_event_id).await;

    let _ = std::fs::remove_dir_all(&store_root);
}

fn rich_message_live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_RICH_MESSAGE_LIVE")
        .ok()
        .as_deref()
        == Some("1")
}

#[tokio::test]
async fn live_native_rich_message_send_against_disposable_synapse_when_configured() {
    if !rich_message_live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let localpart = format!("vs4_{stamp:x}");
    let password = format!("S-{stamp:x}");
    let registration = register_disposable_account(&base, &localpart, &password).await;
    let user_id = registration["user_id"]
        .as_str()
        .expect("registration user_id")
        .to_owned();

    let store_root = temp_store_root();
    let identity = AccountIdentity::new(&user_id, &base).expect("account identity");
    let key = StoreKeyMaterial::generate().expect("store key");
    let config = ClientBuildConfig::product_default(&store_root, identity, Some(key))
        .expect("client config");
    let client = build_unauthenticated_client(&config)
        .await
        .expect("unauthenticated client");
    login_with_password(
        &client,
        &localpart,
        &password,
        &LoginOptions {
            device_display_name: Some("V-SEND.4 rich-message proof".into()),
            request_refresh_token: false,
            device_id: None,
        },
    )
    .await
    .expect("password login");

    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("initial sync");
    let room = client
        .create_room(CreateRoomRequest::new())
        .await
        .expect("create room");
    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("post-create sync");

    let content = crate::matrix::auth::product::message_content(
        "waves".into(),
        Some("m.emote".into()),
        Some("<strong>waves</strong>".into()),
        Some(vec![user_id.clone()]),
        true,
        None,
        None,
    )
    .expect("build native rich message content");
    let response = room
        .send(content)
        .await
        .expect("send rich message via managed client");
    let value = wait_for_message_event(&client, &room, &response.response.event_id).await;
    let content = &value["content"];
    assert_eq!(content["msgtype"], "m.emote");
    assert_eq!(content["format"], "org.matrix.custom.html");
    assert_eq!(content["formatted_body"], "<strong>waves</strong>");
    assert_eq!(content["m.mentions"]["user_ids"][0], user_id);
    assert_eq!(content["m.mentions"]["room"], true);

    let _ = std::fs::remove_dir_all(&store_root);
}

fn thread_send_live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_THREAD_SEND_LIVE")
        .ok()
        .as_deref()
        == Some("1")
}

/// V-SEND.5: native composer thread send against disposable Synapse.
///
/// Root event → in-thread reply via the same `message_content` builder as
/// `matrix_send_text` with `thread_root` + `reply_to`. Verifies wire
/// `m.relates_to` is `m.thread` with the correct root / in_reply_to and
/// non-fallback flag.
#[tokio::test]
async fn live_native_thread_send_against_disposable_synapse_when_configured() {
    if !thread_send_live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let localpart = format!("vs5_{stamp:x}");
    let password = format!("S-{stamp:x}");
    let registration = register_disposable_account(&base, &localpart, &password).await;
    let user_id = registration["user_id"]
        .as_str()
        .expect("registration user_id")
        .to_owned();

    let store_root = temp_store_root();
    let identity = AccountIdentity::new(&user_id, &base).expect("account identity");
    let key = StoreKeyMaterial::generate().expect("store key");
    let config = ClientBuildConfig::product_default(&store_root, identity, Some(key))
        .expect("client config");
    let client = build_unauthenticated_client(&config)
        .await
        .expect("unauthenticated client");
    login_with_password(
        &client,
        &localpart,
        &password,
        &LoginOptions {
            device_display_name: Some("V-SEND.5 thread-send proof".into()),
            request_refresh_token: false,
            device_id: None,
        },
    )
    .await
    .expect("password login");

    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("initial sync");
    let room = client
        .create_room(CreateRoomRequest::new())
        .await
        .expect("create room");
    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("post-create sync");

    let root_content = crate::matrix::auth::product::message_content(
        "thread root".into(),
        Some("m.text".into()),
        None,
        None,
        false,
        None,
        None,
    )
    .expect("build root message");
    let root_response = room
        .send(root_content)
        .await
        .expect("send thread root via managed client");
    let root_event_id = root_response.response.event_id.clone();
    wait_for_message_event(&client, &room, &root_event_id).await;

    // Start thread + genuine reply in one step: root == reply target.
    let thread_content = crate::matrix::auth::product::message_content(
        "thread reply".into(),
        Some("m.text".into()),
        None,
        None,
        false,
        Some(root_event_id.clone()),
        Some(root_event_id.clone()),
    )
    .expect("build in-thread reply content");
    let thread_response = room
        .send(thread_content)
        .await
        .expect("send thread reply via managed client");
    let value = wait_for_message_event(&client, &room, &thread_response.response.event_id).await;
    let relates = &value["content"]["m.relates_to"];
    assert_eq!(relates["rel_type"], "m.thread");
    assert_eq!(relates["event_id"], root_event_id.as_str());
    assert_eq!(relates["m.in_reply_to"]["event_id"], root_event_id.as_str());
    assert!(
        relates
            .get("is_falling_back")
            .map(|v| v == false)
            .unwrap_or(true),
        "thread reply must not be a fallback: {relates}"
    );

    // Reply to the first thread reply, still under the same root.
    let child_event_id = thread_response.response.event_id.clone();
    let nested = crate::matrix::auth::product::message_content(
        "nested reply".into(),
        Some("m.text".into()),
        None,
        None,
        false,
        Some(child_event_id.clone()),
        Some(root_event_id.clone()),
    )
    .expect("build nested in-thread reply");
    let nested_response = room.send(nested).await.expect("send nested thread reply");
    let nested_value =
        wait_for_message_event(&client, &room, &nested_response.response.event_id).await;
    let nested_relates = &nested_value["content"]["m.relates_to"];
    assert_eq!(nested_relates["rel_type"], "m.thread");
    assert_eq!(nested_relates["event_id"], root_event_id.as_str());
    assert_eq!(
        nested_relates["m.in_reply_to"]["event_id"],
        child_event_id.as_str()
    );

    let _ = std::fs::remove_dir_all(&store_root);
}

async fn wait_for_message_event(
    client: &matrix_sdk::Client,
    room: &matrix_sdk::Room,
    event_id: &matrix_sdk::ruma::EventId,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        sync_briefly(client).await;
        match room.load_or_fetch_event(event_id, None).await {
            Ok(ev) => {
                let raw = ev.into_raw();
                let value: serde_json::Value =
                    serde_json::from_str(raw.json().get()).expect("event json");
                assert_eq!(value["type"], "m.room.message");
                return value;
            }
            Err(_) => {
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting to fetch rich message {event_id} from managed room"
                );
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }
}

async fn wait_for_room_event(
    client: &matrix_sdk::Client,
    room: &matrix_sdk::Room,
    event_id: &matrix_sdk::ruma::EventId,
) {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        sync_briefly(client).await;
        match room.load_or_fetch_event(event_id, None).await {
            Ok(ev) => {
                let raw = ev.into_raw();
                let value: serde_json::Value =
                    serde_json::from_str(raw.json().get()).expect("event json");
                let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                assert!(
                    event_type.contains("poll"),
                    "expected poll event type, got {event_type:?}"
                );
                return;
            }
            Err(_) => {
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting to fetch poll event {event_id} from managed room"
                );
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }
}
