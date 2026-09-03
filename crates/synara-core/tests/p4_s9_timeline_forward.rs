//! P4-S9-30: typed SharedCore consume of the registered timeline
//! forward-text / forward-media commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo event id or room id.
//! Backup/crypto/cross-signing/room-key status stay off. No media bytes
//! cross the envelope.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, SharedCore, TimelineForwardDto, TimelineForwardError,
};

struct MemoryCallbackVault(Arc<Mutex<HashMap<String, Vec<u8>>>>);

impl IosSecretVault for MemoryCallbackVault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
        Ok(self.0.lock().expect("vault").get(&key).cloned())
    }

    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
        self.0.lock().expect("vault").insert(key, value);
        Ok(())
    }

    fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
        self.0.lock().expect("vault").remove(&key);
        Ok(())
    }
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-p4-s9-30-it-{tag}-{nanos}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn forward_text_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    source_room_id: String,
    event_id: String,
    target_room_id: String,
    as_quote: bool,
) -> Result<TimelineForwardDto, TimelineForwardError> {
    rt.block_on(shared.timeline_forward_text(
        source_room_id,
        event_id,
        target_room_id,
        as_quote,
        false,
    ))
}

fn forward_media_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    source_room_id: String,
    event_id: String,
    target_room_id: String,
) -> Result<TimelineForwardDto, TimelineForwardError> {
    rt.block_on(shared.timeline_forward_media(source_room_id, event_id, target_room_id, false))
}

fn error_text(error: &TimelineForwardError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn timeline_forward_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_forward_text("));
    assert!(udl.contains("timeline_forward_media("));
    assert!(udl.contains("dictionary TimelineForwardDto"));
    assert!(udl.contains("interface TimelineForwardError"));
    assert!(udl.contains("timeline_poll_vote("));
    assert!(udl.contains("timeline_call_decline("));
    assert!(udl.contains("timeline_pin("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_backup_status"));
    assert!(!udl.contains("matrix_crypto_status"));
    assert!(!udl.contains("matrix_cross_signing_status"));
    assert!(!udl.contains("matrix_cross_signing_setup"));
    assert!(!udl.contains("matrix_room_key_transfer_status"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("timeline_forward_text("));
    assert!(shared_core.contains("timeline_forward_media("));
    assert!(shared_core.contains("timeline_poll_vote("));
    assert!(shared_core.contains("timeline_call_decline("));
    assert!(shared_core.contains("timeline_pin("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));
}

#[test]
fn timeline_forward_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let source_room_id = "!s930SecretSource:example.org";
    let target_room_id = "!s930SecretTarget:example.org";
    let event_id = "$s930SecretEvent:example.org";
    let text = forward_text_plain(
        &rt,
        &shared,
        source_room_id.to_owned(),
        event_id.to_owned(),
        target_room_id.to_owned(),
        false,
    )
    .expect_err("no attached timeline-forward-text owner");
    let media = forward_media_plain(
        &rt,
        &shared,
        source_room_id.to_owned(),
        event_id.to_owned(),
        target_room_id.to_owned(),
    )
    .expect_err("no attached timeline-forward-media owner");
    let text_err = error_text(&text);
    let media_err = error_text(&media);
    assert!(text_err.contains("p2-timeline-forward-text-no-session"));
    assert!(media_err.contains("p2-timeline-forward-media-no-session"));
    let combined = format!("{text_err}{media_err}");
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains("token"));
    assert!(!combined.contains(source_room_id));
    assert!(!combined.contains(target_room_id));
    assert!(!combined.contains(event_id));
    assert!(!combined.contains("@alice"));
}

#[test]
fn timeline_forward_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let source_room_id = format!(
        "!s930OversizeSource{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let target_room_id = format!(
        "!s930OversizeTarget{}:example.org",
        "t".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let event_id = format!(
        "$s930OversizeEvent{}",
        "e".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let text = forward_text_plain(
        &rt,
        &shared,
        source_room_id.clone(),
        event_id.clone(),
        target_room_id.clone(),
        true,
    )
    .expect_err("oversize forward-text payload must fail closed");
    let media = forward_media_plain(
        &rt,
        &shared,
        source_room_id.clone(),
        event_id.clone(),
        target_room_id.clone(),
    )
    .expect_err("oversize forward-media payload must fail closed");
    let text_err = error_text(&text);
    let media_err = error_text(&media);
    assert!(text_err.contains("p4-s9-30-timeline-forward-failed"));
    assert!(media_err.contains("p4-s9-30-timeline-forward-failed"));
    let combined = format!("{text_err}{media_err}");
    assert!(!combined.contains(&source_room_id));
    assert!(!combined.contains(&target_room_id));
    assert!(!combined.contains(&event_id));
    assert!(!combined.contains("s930SecretSource"));
    assert!(!combined.contains("s930SecretTarget"));
    assert!(!combined.contains("s930SecretEvent"));
}

#[test]
fn timeline_forward_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_30_timeline_forward_access";
    let refresh = "syr_s9_30_timeline_forward_refresh";
    let identity = alice();
    let source_room_id = "!s930SecretSource:example.org";
    let target_room_id = "!s930SecretTarget:example.org";
    let event_id = "$s930SecretEvent:example.org";
    let invalid_room = "s930-not-a-room-id";
    let invalid_event = "s930-not-an-event-id";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-forward-no-start");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist");
    rt.block_on(shared.attach_session_owners())
        .expect("owners attached");

    let missing_room_text = forward_text_plain(
        &rt,
        &shared,
        source_room_id.to_owned(),
        event_id.to_owned(),
        target_room_id.to_owned(),
        false,
    );
    let missing_room_media = forward_media_plain(
        &rt,
        &shared,
        source_room_id.to_owned(),
        event_id.to_owned(),
        target_room_id.to_owned(),
    );
    let invalid_room_text = forward_text_plain(
        &rt,
        &shared,
        invalid_room.to_owned(),
        event_id.to_owned(),
        target_room_id.to_owned(),
        false,
    );
    let invalid_event_text = forward_text_plain(
        &rt,
        &shared,
        source_room_id.to_owned(),
        invalid_event.to_owned(),
        target_room_id.to_owned(),
        false,
    );
    let invalid_event_media = forward_media_plain(
        &rt,
        &shared,
        source_room_id.to_owned(),
        invalid_event.to_owned(),
        target_room_id.to_owned(),
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_text_err = missing_room_text
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted forward-text must fail on local room lookup without a live server");
    let missing_room_media_err = missing_room_media
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted forward-media must fail on local room lookup without a live server");
    let invalid_room_text_err = invalid_room_text
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted forward-text must fail on invalid room id without a live server");
    let invalid_event_text_err = invalid_event_text
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted forward-text must fail on invalid event id without a live server");
    let invalid_event_media_err = invalid_event_media
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted forward-media must fail on invalid event id without a live server");

    assert!(
        missing_room_text_err.contains("v-timeline-forward-source-room-not-found"),
        "forward-text must return the registered source-room-not-found diagnostic: {missing_room_text_err}"
    );
    assert!(
        missing_room_media_err.contains("v-timeline-forward-media-source-room-not-found"),
        "forward-media must return the registered source-room-not-found diagnostic: {missing_room_media_err}"
    );
    assert!(
        invalid_room_text_err.contains("d0.4-send-invalid-room-id"),
        "forward-text must return the registered invalid-room diagnostic: {invalid_room_text_err}"
    );
    assert!(
        invalid_event_text_err.contains("v-timeline-forward-invalid-event-id"),
        "forward-text must return the registered invalid-event diagnostic: {invalid_event_text_err}"
    );
    assert!(
        invalid_event_media_err.contains("v-timeline-forward-media-invalid-event-id"),
        "forward-media must return the registered invalid-event diagnostic: {invalid_event_media_err}"
    );
    for (label, text) in [
        ("missing_room_text", &missing_room_text_err),
        ("missing_room_media", &missing_room_media_err),
        ("invalid_room_text", &invalid_room_text_err),
        ("invalid_event_text", &invalid_event_text_err),
        ("invalid_event_media", &invalid_event_media_err),
    ] {
        assert!(
            !text.contains("p4-s9-30-timeline-forward-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let combined = format!(
        "{missing_room_text_err}{missing_room_media_err}{invalid_room_text_err}{invalid_event_text_err}{invalid_event_media_err}"
    );
    assert!(!combined.contains(access));
    assert!(!combined.contains(refresh));
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains(source_room_id));
    assert!(!combined.contains(target_room_id));
    assert!(!combined.contains(event_id));
    assert!(!combined.contains(invalid_room));
    assert!(!combined.contains(invalid_event));
}
