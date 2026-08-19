//! Opt-in live V-SEND.2 proof against disposable Synapse.
//!
//! Path marker `.../tests.rs` keeps this file outside production matrix
//! client/sync guardrails while remaining crate-private.
//!
//! Gated by:
//! - `SYNARA_RUN_MATRIX_RUST_REACTION_LIVE=1`
//! - `SYNARA_MATRIX_HOMESERVER_URL=http://127.0.0.1:<port>` (credential-free HTTP loopback)
//!
//! Exercises the native owner route end-to-end:
//! register/login → create room → open native timeline → send target (so the
//! live timeline observes local/remote echo) →
//! `NativeTimelineRegistry::{toggle,ensure,redact}_reaction` → aggregation
//! readback.
//!
//! JS two-client Synapse CI is not this proof. WebView click-through is not required.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use sha1::Sha1;

use crate::matrix::auth::{login_with_password, LoginOptions};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::store::{AccountIdentity, StoreKeyMaterial};
use crate::matrix::timeline::live::{
    NativeReactionMutation, NativeTimelineDirection, NativeTimelineReaction, NativeTimelineRegistry,
};

type HmacSha1 = Hmac<Sha1>;

const TARGET_BODY: &str = "v-send.2 reaction proof target";

fn live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_REACTION_LIVE")
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
    let root = std::env::temp_dir().join(format!("synara-v-send2-reaction-{nanos}"));
    std::fs::create_dir_all(&root).expect("temp store root");
    root
}

async fn sync_briefly(client: &matrix_sdk::Client) {
    let _ = client
        .sync_once(SyncSettings::default().timeout(Duration::from_secs(1)))
        .await;
}

/// Keep one live registry open so send-queue local echo and sync-fed remote
/// echo can land. Cold reopen after pre-subscribe sync leaves the event cache
/// empty and `Timeline::toggle_reaction` cannot find the target.
async fn wait_for_target_in_open_timeline(
    registry: &mut NativeTimelineRegistry,
    client: &matrix_sdk::Client,
    room_id: &str,
    event_id: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        sync_briefly(client).await;
        // Cold event-cache rooms need a /messages page before the target appears.
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
            "timed out waiting for target event {event_id} in open native timeline (items={} bodies={:?})",
            snapshot.items.len(),
            snapshot
                .items
                .iter()
                .map(|item| item.body.as_str())
                .collect::<Vec<_>>(),
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn poll_me_reaction(
    registry: &mut NativeTimelineRegistry,
    client: &matrix_sdk::Client,
    room_id: &str,
    event_id: &str,
    key: &str,
    want_me: bool,
) -> Option<NativeTimelineReaction> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        sync_briefly(client).await;
        let snapshot = registry
            .snapshot(client, room_id)
            .await
            .expect("timeline snapshot");
        let readback = snapshot
            .items
            .into_iter()
            .find(|item| item.event_id == event_id)
            .and_then(|item| {
                item.reactions
                    .into_iter()
                    .find(|reaction| reaction.key == key)
            });
        if readback.as_ref().is_some_and(|r| r.me == want_me) || (readback.is_none() && !want_me) {
            return readback;
        }
        if Instant::now() >= deadline {
            return readback;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn live_native_reaction_paths_against_disposable_synapse_when_configured() {
    if !live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let localpart = format!("vs2_{stamp:x}");
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
            device_display_name: Some("V-SEND.2 reaction proof".into()),
            request_refresh_token: false,
            device_id: None,
        },
    )
    .await
    .expect("password login");

    // Subscribe before sync so timeline-linked chunks observe room traffic.
    client
        .event_cache()
        .subscribe()
        .expect("subscribe event cache for native reaction proof");

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

    // Open the live native timeline *before* sending so toggle_reaction can see
    // the target via send-queue echo / cache updates (not a cold reopen).
    let mut registry = NativeTimelineRegistry::new(1);
    registry
        .open(&client, &room_id)
        .await
        .expect("open native timeline before send");

    let sent = room
        .send(RoomMessageEventContent::text_plain(TARGET_BODY))
        .await
        .expect("send target event");
    let target_event_id = sent.response.event_id.to_string();

    wait_for_target_in_open_timeline(&mut registry, &client, &room_id, &target_event_id).await;

    // Path 1: toggle add — requires the target event already present in the timeline.
    let toggled = registry
        .toggle_reaction(&client, &room_id, &target_event_id, "✅")
        .await
        .expect("toggle add");
    assert_eq!(toggled.mutation, NativeReactionMutation::Added);
    let after_toggle = poll_me_reaction(
        &mut registry,
        &client,
        &room_id,
        &target_event_id,
        "✅",
        true,
    )
    .await
    .expect("toggle aggregation readback");
    assert!(
        after_toggle.me,
        "toggle must mark me=true in native readback"
    );
    assert!(after_toggle.count >= 1);

    // Path 2: ensure is idempotent (never toggles away)
    let ensured = registry
        .ensure_reaction(&client, &room_id, &target_event_id, "✅")
        .await
        .expect("ensure already present");
    assert_eq!(ensured.mutation, NativeReactionMutation::AlreadyPresent);
    assert!(ensured.readback.as_ref().is_some_and(|r| r.me));

    // Path 3: redact selected annotation once Synapse assigns a remote event id.
    let reaction_event_id = {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut found = after_toggle
            .senders
            .iter()
            .find_map(|sender| sender.reaction_event_id.clone());
        while found.is_none() && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(200)).await;
            found = poll_me_reaction(
                &mut registry,
                &client,
                &room_id,
                &target_event_id,
                "✅",
                true,
            )
            .await
            .and_then(|r| {
                r.senders
                    .into_iter()
                    .find_map(|sender| sender.reaction_event_id)
            });
        }
        found.expect("remote reaction event id for redaction")
    };

    let redacted = registry
        .redact_reaction(
            &client,
            &room_id,
            &target_event_id,
            &reaction_event_id,
            "✅",
        )
        .await
        .expect("redact annotation");
    assert_eq!(redacted.mutation, NativeReactionMutation::Redacted);
    let after_redact = poll_me_reaction(
        &mut registry,
        &client,
        &room_id,
        &target_event_id,
        "✅",
        false,
    )
    .await;
    assert!(
        after_redact.as_ref().is_none_or(|r| !r.me),
        "redact must clear me=true from native aggregation readback"
    );

    // Ensure can re-add after redaction (distinct from toggle remove)
    let readded = registry
        .ensure_reaction(&client, &room_id, &target_event_id, "✅")
        .await
        .expect("ensure re-add");
    assert_eq!(readded.mutation, NativeReactionMutation::Added);
    let after_ensure = poll_me_reaction(
        &mut registry,
        &client,
        &room_id,
        &target_event_id,
        "✅",
        true,
    )
    .await
    .expect("ensure aggregation readback");
    assert!(after_ensure.me);

    let _ = std::fs::remove_dir_all(&store_root);
}
