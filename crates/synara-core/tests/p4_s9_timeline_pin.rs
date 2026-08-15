//! P4-S9-28: typed SharedCore consume of the registered timeline
//! pin / unpin commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo event id or room id.
//! Poll vote / call decline stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, SharedCore, TimelinePinDto, TimelinePinError,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-28-it-{tag}-{nanos}"));
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

fn pin_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
) -> Result<TimelinePinDto, TimelinePinError> {
    rt.block_on(shared.timeline_pin(room_id, event_id))
}

fn unpin_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
) -> Result<TimelinePinDto, TimelinePinError> {
    rt.block_on(shared.timeline_unpin(room_id, event_id))
}

fn error_text(error: &TimelinePinError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn timeline_pin_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_pin("));
    assert!(udl.contains("timeline_unpin("));
    assert!(udl.contains("dictionary TimelinePinDto"));
    assert!(udl.contains("interface TimelinePinError"));
    assert!(udl.contains("timeline_edit_text("));
    assert!(udl.contains("timeline_redact("));
    assert!(udl.contains("timeline_report("));
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
    assert!(shared_core.contains("timeline_pin("));
    assert!(shared_core.contains("timeline_unpin("));
    assert!(shared_core.contains("timeline_edit_text("));
    assert!(shared_core.contains("timeline_redact("));
    assert!(shared_core.contains("timeline_report("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));
    assert!(!shared_core.contains("cross_signing_status"));
    assert!(!shared_core.contains("cross_signing_setup"));
    assert!(!shared_core.contains("room_key_transfer_status"));
}

#[test]
fn timeline_pin_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s928SecretRoom:example.org";
    let event_id = "$s928SecretEvent:example.org";
    let pin = pin_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned())
        .expect_err("no attached timeline-pin owner");
    let unpin = unpin_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned())
        .expect_err("no attached timeline-unpin owner");
    let pin_text = error_text(&pin);
    let unpin_text = error_text(&unpin);
    assert!(pin_text.contains("p2-timeline-pin-no-session"));
    assert!(unpin_text.contains("p2-timeline-unpin-no-session"));
    let text = format!("{pin_text}{unpin_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn timeline_pin_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = format!(
        "!s928OversizeRoom{}:example.org",
        "r".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let event_id = format!(
        "$s928OversizeEvent{}",
        "e".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let pin = pin_plain(&rt, &shared, room_id.clone(), event_id.clone())
        .expect_err("oversize pin payload must fail closed");
    let unpin = unpin_plain(&rt, &shared, room_id.clone(), event_id.clone())
        .expect_err("oversize unpin payload must fail closed");
    let pin_text = error_text(&pin);
    let unpin_text = error_text(&unpin);
    assert!(pin_text.contains("p4-s9-28-timeline-pin-failed"));
    assert!(unpin_text.contains("p4-s9-28-timeline-pin-failed"));
    let text = format!("{pin_text}{unpin_text}");
    assert!(!text.contains(&room_id));
    assert!(!text.contains(&event_id));
    assert!(!text.contains("s928SecretRoom"));
    assert!(!text.contains("s928SecretEvent"));
}

#[test]
fn timeline_pin_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_28_timeline_pin_access";
    let refresh = "syr_s9_28_timeline_pin_refresh";
    let identity = alice();
    let room_id = "!s928SecretRoom:example.org";
    let event_id = "$s928SecretEvent:example.org";
    let invalid_room = "s928-not-a-room-id";
    let invalid_event = "s928-not-an-event-id";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-pin-no-start");
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

    let missing_room_pin = pin_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned());
    let missing_room_unpin = unpin_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned());
    let invalid_room_pin = pin_plain(&rt, &shared, invalid_room.to_owned(), event_id.to_owned());
    let invalid_event_pin = pin_plain(&rt, &shared, room_id.to_owned(), invalid_event.to_owned());
    let invalid_event_unpin =
        unpin_plain(&rt, &shared, room_id.to_owned(), invalid_event.to_owned());
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_pin_text = missing_room_pin
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted pin must fail on local room lookup without a live server");
    let missing_room_unpin_text = missing_room_unpin
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted unpin must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_pin
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted pin must fail on invalid room id without a live server");
    let invalid_event_pin_text = invalid_event_pin
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted pin must fail on invalid event id without a live server");
    let invalid_event_unpin_text = invalid_event_unpin
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted unpin must fail on invalid event id without a live server");

    assert!(
        missing_room_pin_text.contains("v-timeline-pin-room-not-found"),
        "pin must return the registered room-not-found diagnostic: {missing_room_pin_text}"
    );
    assert!(
        missing_room_unpin_text.contains("v-timeline-unpin-room-not-found"),
        "unpin must return the registered room-not-found diagnostic: {missing_room_unpin_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "pin must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_pin_text.contains("v-timeline-pin-invalid-event-id"),
        "pin must return the registered invalid-event diagnostic: {invalid_event_pin_text}"
    );
    assert!(
        invalid_event_unpin_text.contains("v-timeline-unpin-invalid-event-id"),
        "unpin must return the registered invalid-event diagnostic: {invalid_event_unpin_text}"
    );
    for (label, text) in [
        ("missing_room_pin", &missing_room_pin_text),
        ("missing_room_unpin", &missing_room_unpin_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_event_pin", &invalid_event_pin_text),
        ("invalid_event_unpin", &invalid_event_unpin_text),
    ] {
        assert!(
            !text.contains("p4-s9-28-timeline-pin-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!(
        "{missing_room_pin_text}{missing_room_unpin_text}{invalid_room_text}{invalid_event_pin_text}{invalid_event_unpin_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
}
