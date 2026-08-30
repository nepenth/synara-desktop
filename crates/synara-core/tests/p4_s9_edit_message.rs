//! P4-S9-25: typed SharedCore consume of the registered edit-message command.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! No media bytes. Failed errors stay static and must not echo body,
//! event id, or room id. Timeline edit/redact/report stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    EditMessageDto, EditMessageError, IosSecretVault, IosSecretVaultError, SharedCore,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-25-it-{tag}-{nanos}"));
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

fn edit_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
    body: String,
) -> Result<EditMessageDto, EditMessageError> {
    rt.block_on(shared.edit_message(room_id, event_id, body, None, None, None, None, None))
}

fn error_text(error: &EditMessageError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn edit_message_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("edit_message("));
    assert!(udl.contains("dictionary EditMessageDto"));
    assert!(udl.contains("interface EditMessageError"));
    assert!(udl.contains("send_poll("));
    assert!(!udl.contains("send_sticker("));
    assert!(udl.contains("send_text("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("edit_message("));
    assert!(shared_core.contains("send_poll("));
    assert!(!shared_core.contains("send_sticker("));
    assert!(shared_core.contains("send_text("));
    assert!(shared_core.contains("composer_set_reply_draft("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn edit_message_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s925SecretRoom:example.org";
    let event_id = "$s925SecretEvent:example.org";
    let body = "s925SecretBody";
    let error = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        body.to_owned(),
    )
    .expect_err("no attached edit-message owner");
    let text = error_text(&error);
    assert!(text.contains("p2-edit-message-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(body));
    assert!(!text.contains("@alice"));
}

#[test]
fn edit_message_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s925OversizeRoom:example.org";
    let event_id = "$s925OversizeEvent:example.org";
    let body = format!(
        "s925OversizeBody{}",
        "b".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let error = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        body.clone(),
    )
    .expect_err("oversize edit-message payload must fail closed");
    let text = error_text(&error);
    assert!(text.contains("p4-s9-25-edit-message-failed"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(&body));
    assert!(!text.contains("s925SecretBody"));
}

#[test]
fn edit_message_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_25_edit_message_access";
    let refresh = "syr_s9_25_edit_message_refresh";
    let identity = alice();
    let room_id = "!s925SecretRoom:example.org";
    let event_id = "$s925SecretEvent:example.org";
    let body = "s925SecretBody";
    let invalid_room = "s925-not-a-room-id";
    let invalid_event = "s925-not-an-event-id";
    let invalid_msg_type = "s925-not-a-msg-type";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("edit-message-no-start");
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

    let missing_room = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        body.to_owned(),
    );
    let invalid_room_edit = edit_plain(
        &rt,
        &shared,
        invalid_room.to_owned(),
        event_id.to_owned(),
        body.to_owned(),
    );
    let invalid_event_edit = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        invalid_event.to_owned(),
        body.to_owned(),
    );
    let invalid_type_edit = rt.block_on(shared.edit_message(
        room_id.to_owned(),
        event_id.to_owned(),
        body.to_owned(),
        Some(invalid_msg_type.to_owned()),
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
        .expect("planted edit must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on invalid room id without a live server");
    let invalid_event_text = invalid_event_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on invalid event id without a live server");
    let invalid_type_text = invalid_type_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on invalid message type without a live server");

    assert!(
        missing_room_text.contains("v-send.r-edit-room-not-found"),
        "edit must return the registered room-not-found diagnostic: {missing_room_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "edit must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_text.contains("v-send.r-edit-invalid-event-id"),
        "edit must return the registered invalid-event diagnostic: {invalid_event_text}"
    );
    assert!(
        invalid_type_text.contains("v-send.4-invalid-message-type"),
        "edit must return the registered invalid-type diagnostic: {invalid_type_text}"
    );
    for (label, text) in [
        ("missing_room", &missing_room_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_event", &invalid_event_text),
        ("invalid_type", &invalid_type_text),
    ] {
        assert!(
            !text.contains("p4-s9-25-edit-message-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text =
        format!("{missing_room_text}{invalid_room_text}{invalid_event_text}{invalid_type_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(body));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
    assert!(!text.contains(invalid_msg_type));
}
