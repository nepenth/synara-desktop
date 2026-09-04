//! Opt-in native two-client receipt + ordered-delivery proof against disposable
//! Synapse — the native replacement for the retired js-sdk two-client harness.
//!
//! Path marker `.../receipts.rs` keeps this file outside production matrix
//! client/sync guardrails while remaining crate-private.
//!
//! Gated by:
//! - `SYNARA_RUN_MATRIX_RUST_RECEIPT_LIVE=1`
//! - `SYNARA_MATRIX_HOMESERVER_URL=http://127.0.0.1:<port>` (credential-free HTTP loopback)
//!
//! Validates on the **native** product path what the js-sdk two-client harness
//! used to validate on the legacy client:
//! 1. **Ordered delivery** — client A sends N numbered events; client B reads
//!    them back through the product `NativeTimelineRegistry` and must observe a
//!    strictly increasing sequence.
//! 2. **Receipt visibility** — client A sends a public read receipt + fully-read
//!    marker on the last event; client B observes A's `m.read` receipt pointing
//!    at exactly that event, and A observes its own fully-read marker.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit, Mac};
use matrix_sdk::config::SyncSettings;
use matrix_sdk::room::Receipts;
use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
use matrix_sdk::ruma::events::fully_read::FullyReadEventContent;
use matrix_sdk::ruma::events::receipt::{ReceiptThread, ReceiptType};
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::{OwnedEventId, OwnedUserId};
use sha1::Sha1;

use crate::matrix::auth::{login_with_password, LoginOptions};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::store::{AccountIdentity, StoreKeyMaterial};
use crate::matrix::timeline::live::{NativeTimelineDirection, NativeTimelineRegistry};

type HmacSha1 = Hmac<Sha1>;

const EVENT_COUNT: usize = 5;
const SEQ_PREFIX: &str = "v-burn-two-client-receipt-";

fn live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_RECEIPT_LIVE")
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

fn temp_store_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-receipt-proof-{label}-{nanos}"));
    std::fs::create_dir_all(&root).expect("temp store root");
    root
}

async fn sync_briefly(client: &matrix_sdk::Client) {
    let _ = client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(1)))
        .await;
}

/// Register + login a disposable native client on the harness Synapse.
async fn build_client(
    base: &str,
    stamp: u128,
    tag: &str,
    store_label: &str,
) -> (matrix_sdk::Client, OwnedUserId, PathBuf) {
    let localpart = format!("rcpt_{tag}_{stamp:x}");
    let password = format!("R-{stamp:x}");
    let registration = register_disposable_account(base, &localpart, &password).await;
    let user_id: OwnedUserId = registration["user_id"]
        .as_str()
        .expect("registration user_id")
        .parse()
        .expect("parse user id");

    let store_root = temp_store_root(store_label);
    let identity = AccountIdentity::new(user_id.as_str(), base).expect("account identity");
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
            device_display_name: Some(format!("receipt proof {tag}")),
            request_refresh_token: false,
            device_id: None,
        },
    )
    .await
    .expect("password login");

    client
        .event_cache()
        .subscribe()
        .expect("subscribe event cache");

    client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(3)))
        .await
        .expect("initial sync");
    (client, user_id, store_root)
}

#[tokio::test]
async fn live_native_two_client_receipt_and_ordering_against_disposable_synapse_when_configured() {
    if !live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();

    // Two independent native clients = two users/devices.
    let (client_a, user_a, store_a) = build_client(&base, stamp, "a", "a").await;
    let (client_b, user_b, store_b) = build_client(&base, stamp, "b", "b").await;

    // A creates a room, invites B, B joins.
    let room = client_a
        .create_room(CreateRoomRequest::new())
        .await
        .expect("A create room");
    let room_id = room.room_id().to_owned();
    room.invite_user_by_id(&user_b).await.expect("A invite B");
    client_b
        .join_room_by_id(room.room_id())
        .await
        .expect("B join room");
    sync_briefly(&client_a).await;
    sync_briefly(&client_b).await;

    // A sends EVENT_COUNT ordered events.
    let mut event_ids = Vec::new();
    for index in 1..=EVENT_COUNT {
        let sent = room
            .send(RoomMessageEventContent::text_plain(format!(
                "{SEQ_PREFIX}{index}"
            )))
            .await
            .expect("A send sequenced event");
        event_ids.push(sent.response.event_id);
    }
    let last_event_id: OwnedEventId = event_ids.last().expect("last event id").clone();

    // B opens the product-native timeline and must observe every event in strict order.
    let mut registry_b = NativeTimelineRegistry::new(1);
    registry_b
        .open(&client_b, room_id.as_str())
        .await
        .expect("B open native timeline before readback");
    let ordering_deadline = Instant::now() + Duration::from_secs(45);
    loop {
        sync_briefly(&client_b).await;
        let _ = registry_b
            .paginate_legacy(
                &client_b,
                room_id.as_str(),
                NativeTimelineDirection::Backwards,
            )
            .await;
        let snapshot = registry_b
            .snapshot(&client_b, room_id.as_str())
            .await
            .expect("B native timeline snapshot");
        let observed: Vec<usize> = snapshot
            .items
            .iter()
            .filter_map(|item| {
                item.body
                    .strip_prefix(SEQ_PREFIX)
                    .and_then(|rest| rest.parse::<usize>().ok())
            })
            .collect();
        let mut unique = observed.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() == EVENT_COUNT {
            assert!(
                observed.windows(2).all(|pair| pair[0] < pair[1]),
                "B must observe strictly ordered delivery, got {observed:?}"
            );
            break;
        }
        assert!(
            Instant::now() < ordering_deadline,
            "timed out waiting for {EVENT_COUNT} ordered events on B (seen {unique:?}, bodies={:?})",
            snapshot
                .items
                .iter()
                .map(|item| item.body.as_str())
                .collect::<Vec<_>>(),
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // A marks everything read: public read receipt + fully-read marker on the last event.
    room.send_multiple_receipts(
        Receipts::new()
            .public_read_receipt(last_event_id.clone())
            .fully_read_marker(last_event_id.clone()),
    )
    .await
    .expect("A send read receipts");

    // B observes A's public read receipt pointing at exactly the last event.
    let room_b = client_b.get_room(&room_id).expect("B room handle");
    let receipt_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        sync_briefly(&client_b).await;
        if let Ok(Some((observed_event, _))) = room_b
            .load_user_receipt(ReceiptType::Read, ReceiptThread::Unthreaded, &user_a)
            .await
        {
            assert_eq!(
                observed_event.to_string(),
                last_event_id.to_string(),
                "B must observe A read receipt on the final event"
            );
            break;
        }
        assert!(
            Instant::now() < receipt_deadline,
            "timed out waiting for A read receipt on B"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // A also observes its own fully-read marker (room account data). The marker
    // is set server-side via set_read_marker, so A must pull it back over sync.
    let room_a = client_a.get_room(&room_id).expect("A room handle");
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    let mut fully_read = room_a
        .account_data_static::<FullyReadEventContent>()
        .await
        .expect("A fully-read account data load");
    while fully_read.is_none() && Instant::now() < marker_deadline {
        sync_briefly(&client_a).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
        fully_read = room_a
            .account_data_static::<FullyReadEventContent>()
            .await
            .expect("A fully-read account data poll");
    }
    let marker_event_id = fully_read
        .expect("A must have a fully-read marker after sending read receipts")
        .deserialize()
        .expect("deserialize fully-read room account data")
        .content;
    assert_eq!(
        marker_event_id.event_id.to_string(),
        last_event_id.to_string(),
        "A must observe its own fully-read marker on the final event"
    );

    let _ = std::fs::remove_dir_all(&store_a);
    let _ = std::fs::remove_dir_all(&store_b);
}
