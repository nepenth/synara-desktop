//! P4-S9-19: typed SharedCore consume of the three registered timeline
//! read-state commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Jump returns the existing open readback. Do not re-wrap S6 open.
//! Failed errors stay static and must not echo event id, room id, or
//! stream id. Composer reply draft stays off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-19-it-{tag}-{nanos}"));
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

#[test]
fn timeline_read_state_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_event_readback("));
    assert!(udl.contains("timeline_set_read_state("));
    assert!(udl.contains("timeline_jump_latest("));
    assert!(udl.contains("dictionary TimelineEventReadbackDto"));
    assert!(udl.contains("dictionary TimelineReadStateDto"));
    assert!(udl.contains("dictionary TimelineOpenDto"));
    assert!(udl.contains("interface TimelineReadStateError"));
    assert!(udl.contains("timeline_open("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_send_sticker"));
    assert!(!udl.contains("matrix_send_poll"));
    assert!(!udl.contains("matrix_edit_message"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("timeline_event_readback("));
    assert!(shared_core.contains("timeline_set_read_state("));
    assert!(shared_core.contains("timeline_jump_latest("));
    assert!(shared_core.contains("invites_accept("));
    assert!(shared_core.contains("timeline_open("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("send_sticker"));
    assert!(!shared_core.contains("send_poll"));
    assert!(!shared_core.contains("edit_message"));
    assert!(!shared_core.contains("poll_respond"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn timeline_read_state_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s919SecretRoom:example.org";
    let event_id = "$s919SecretEvent";
    let stream_id = "s919SecretStream";
    let readback = rt
        .block_on(shared.timeline_event_readback(room_id.to_owned(), event_id.to_owned()))
        .expect_err("no attached event-readback owner");
    let set_read = rt
        .block_on(shared.timeline_set_read_state(stream_id.to_owned(), "mark_read".to_owned()))
        .expect_err("no attached set-read-state owner");
    let jump = rt
        .block_on(shared.timeline_jump_latest(stream_id.to_owned()))
        .expect_err("no attached jump-latest owner");
    let readback_text = format!("{readback:?}{readback}");
    let set_read_text = format!("{set_read:?}{set_read}");
    let jump_text = format!("{jump:?}{jump}");
    assert!(readback_text.contains("p2-timeline-event-readback-no-session"));
    assert!(set_read_text.contains("p2-timeline-set-read-state-no-session"));
    assert!(jump_text.contains("p2-timeline-jump-latest-no-session"));
    let text = format!("{readback_text}{set_read_text}{jump_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(stream_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn timeline_read_state_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let event_id = format!("${}", "e".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8));
    let stream_id = "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let rt = test_runtime();
    let readback = rt
        .block_on(shared.timeline_event_readback(room_id.clone(), event_id.clone()))
        .expect_err("oversize event-readback payload must fail closed");
    let set_read = rt
        .block_on(shared.timeline_set_read_state(stream_id.clone(), "mark_read".to_owned()))
        .expect_err("oversize set-read-state payload must fail closed");
    let jump = rt
        .block_on(shared.timeline_jump_latest(stream_id.clone()))
        .expect_err("oversize jump-latest payload must fail closed");
    let readback_text = format!("{readback:?}{readback}");
    let set_read_text = format!("{set_read:?}{set_read}");
    let jump_text = format!("{jump:?}{jump}");
    assert!(readback_text.contains("p4-s9-19-timeline-read-state-failed"));
    assert!(set_read_text.contains("p4-s9-19-timeline-read-state-failed"));
    assert!(jump_text.contains("p4-s9-19-timeline-read-state-failed"));
    assert!(!readback_text.contains(&room_id));
    assert!(!readback_text.contains(&event_id));
    assert!(!set_read_text.contains(&stream_id));
    assert!(!jump_text.contains(&stream_id));
}

#[test]
fn timeline_read_state_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_19_timeline_read_state_access";
    let refresh = "syr_s9_19_timeline_read_state_refresh";
    let identity = alice();
    let room_id = "!s919SecretRoom:example.org";
    let event_id = "$s919SecretEvent";
    let invalid_room = "s919-not-a-room-id";
    let invalid_event = "s919-not-an-event-id";
    let stream_id = "s919SecretStream";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-read-state-no-start");
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

    let missing_room =
        rt.block_on(shared.timeline_event_readback(room_id.to_owned(), event_id.to_owned()));
    let invalid_room_id =
        rt.block_on(shared.timeline_event_readback(invalid_room.to_owned(), event_id.to_owned()));
    let invalid_event_id =
        rt.block_on(shared.timeline_event_readback(room_id.to_owned(), invalid_event.to_owned()));
    let set_read =
        rt.block_on(shared.timeline_set_read_state(stream_id.to_owned(), "mark_read".to_owned()));
    let jump = rt.block_on(shared.timeline_jump_latest(stream_id.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_text = missing_room
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted event-readback must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_id
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted event-readback must fail on invalid room id without a live server");
    let invalid_event_text = invalid_event_id
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted event-readback must fail on invalid event id without a live server");
    let set_read_text = set_read
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set-read-state must fail on local stream lookup without a live server");
    let jump_text = jump
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted jump-latest must fail on local stream lookup without a live server");

    assert!(
        missing_room_text.contains("v-crypto.6-event-room-not-found"),
        "event-readback must return the registered room-not-found diagnostic: {missing_room_text}"
    );
    assert!(
        invalid_room_text.contains("d0.3-timeline-invalid-room-id"),
        "event-readback must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_text.contains("v-crypto.6-invalid-event-id"),
        "event-readback must return the registered invalid-event diagnostic: {invalid_event_text}"
    );
    for (label, text) in [("set_read", &set_read_text), ("jump", &jump_text)] {
        assert!(
            text.contains("v-timeline-view-not-open"),
            "{label} must return the registered view-not-open diagnostic: {text}"
        );
        assert!(
            !text.contains("p4-s9-19-timeline-read-state-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    for (label, text) in [
        ("missing_room", &missing_room_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_event", &invalid_event_text),
    ] {
        assert!(
            !text.contains("p4-s9-19-timeline-read-state-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!(
        "{missing_room_text}{invalid_room_text}{invalid_event_text}{set_read_text}{jump_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
    assert!(!text.contains(stream_id));
}
