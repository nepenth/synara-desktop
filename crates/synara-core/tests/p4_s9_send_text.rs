//! P4-S9-22: typed SharedCore consume of the registered send-text command.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! Failed errors stay static and must not echo body or room id.
//! Timeline edit/redact/report stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, SendTextDto, SendTextError, SharedCore};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-22-it-{tag}-{nanos}"));
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

fn send_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    body: String,
) -> Result<SendTextDto, SendTextError> {
    rt.block_on(shared.send_text(room_id, body, None, None, None, None, None, None, None))
}

fn error_text(error: &SendTextError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn send_text_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("send_text("));
    assert!(udl.contains("dictionary SendTextDto"));
    assert!(udl.contains("interface SendTextError"));
    assert!(udl.contains("composer_set_reply_draft("));
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
    assert!(shared_core.contains("send_text("));
    assert!(shared_core.contains("composer_set_reply_draft("));
    assert!(shared_core.contains("reaction_ensure("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn send_text_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s922SecretRoom:example.org";
    let body = "s922SecretBody";
    let error = send_plain(&rt, &shared, room_id.to_owned(), body.to_owned())
        .expect_err("no attached send-text owner");
    let text = error_text(&error);
    assert!(text.contains("p2-send-text-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(body));
    assert!(!text.contains("@alice"));
}

#[test]
fn send_text_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s922OversizeRoom:example.org";
    let body = "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let error = send_plain(&rt, &shared, room_id.to_owned(), body.clone())
        .expect_err("oversize send-text payload must fail closed");
    let text = error_text(&error);
    assert!(text.contains("p4-s9-22-send-text-failed"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(&body));
    assert!(!text.contains("s922SecretBody"));
}

#[test]
fn send_text_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_22_send_text_access";
    let refresh = "syr_s9_22_send_text_refresh";
    let identity = alice();
    let room_id = "!s922SecretRoom:example.org";
    let body = "s922SecretBody";
    let invalid_room = "s922-not-a-room-id";
    let invalid_reply = "s922-not-an-event-id";
    let invalid_msg_type = "m.s922-not-a-type";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("send-text-no-start");
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

    let missing_room = send_plain(&rt, &shared, room_id.to_owned(), body.to_owned());
    let invalid_room_send = send_plain(&rt, &shared, invalid_room.to_owned(), body.to_owned());
    let invalid_reply_send = rt.block_on(shared.send_text(
        room_id.to_owned(),
        body.to_owned(),
        None,
        None,
        None,
        None,
        Some(invalid_reply.to_owned()),
        None,
        None,
    ));
    let invalid_type_send = rt.block_on(shared.send_text(
        room_id.to_owned(),
        body.to_owned(),
        Some(invalid_msg_type.to_owned()),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_text = missing_room
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_send
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on invalid room id without a live server");
    let invalid_reply_text = invalid_reply_send
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on invalid reply id without a live server");
    let invalid_type_text = invalid_type_send
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on invalid message type without a live server");

    assert!(
        missing_room_text.contains("d0.4-send-room-not-found"),
        "send must return the registered room-not-found diagnostic: {missing_room_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "send must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_reply_text.contains("d0.4-send-invalid-reply-event-id"),
        "send must return the registered invalid-reply diagnostic: {invalid_reply_text}"
    );
    assert!(
        invalid_type_text.contains("v-send.4-invalid-message-type"),
        "send must return the registered invalid-type diagnostic: {invalid_type_text}"
    );
    for (label, text) in [
        ("missing_room", &missing_room_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_reply", &invalid_reply_text),
        ("invalid_type", &invalid_type_text),
    ] {
        assert!(
            !text.contains("p4-s9-22-send-text-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text =
        format!("{missing_room_text}{invalid_room_text}{invalid_reply_text}{invalid_type_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(body));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_reply));
    assert!(!text.contains(invalid_msg_type));
}
