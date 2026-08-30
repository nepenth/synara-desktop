//! P4-S9-26: typed SharedCore consume of the registered poll-respond command.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! No media bytes. Failed errors stay static and must not echo answers,
//! event id, or room id. Timeline edit/redact/report stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, PollRespondDto, PollRespondError, SharedCore,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-26-it-{tag}-{nanos}"));
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

fn respond_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    poll_event_id: String,
    answer_ids: Vec<String>,
) -> Result<PollRespondDto, PollRespondError> {
    rt.block_on(shared.poll_respond(room_id, poll_event_id, answer_ids))
}

fn error_text(error: &PollRespondError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn poll_respond_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("poll_respond("));
    assert!(udl.contains("dictionary PollRespondDto"));
    assert!(udl.contains("interface PollRespondError"));
    assert!(udl.contains("edit_message("));
    assert!(udl.contains("send_poll("));
    assert!(!udl.contains("send_sticker("));
    assert!(udl.contains("send_text("));
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
    assert!(shared_core.contains("poll_respond("));
    assert!(shared_core.contains("edit_message("));
    assert!(shared_core.contains("send_poll("));
    assert!(!shared_core.contains("send_sticker("));
    assert!(shared_core.contains("send_text("));
    assert!(shared_core.contains("composer_set_reply_draft("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));
}

#[test]
fn poll_respond_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s926SecretRoom:example.org";
    let poll_event_id = "$s926SecretEvent:example.org";
    let answer = "s926SecretAnswer";
    let error = respond_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        poll_event_id.to_owned(),
        vec![answer.to_owned()],
    )
    .expect_err("no attached poll-respond owner");
    let text = error_text(&error);
    assert!(text.contains("p2-poll-respond-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(poll_event_id));
    assert!(!text.contains(answer));
    assert!(!text.contains("@alice"));
}

#[test]
fn poll_respond_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s926OversizeRoom:example.org";
    let poll_event_id = "$s926OversizeEvent:example.org";
    let answer = format!(
        "s926OversizeAnswer{}",
        "a".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let error = respond_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        poll_event_id.to_owned(),
        vec![answer.clone()],
    )
    .expect_err("oversize poll-respond payload must fail closed");
    let text = error_text(&error);
    assert!(text.contains("p4-s9-26-poll-respond-failed"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(poll_event_id));
    assert!(!text.contains(&answer));
    assert!(!text.contains("s926SecretAnswer"));
}

#[test]
fn poll_respond_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_26_poll_respond_access";
    let refresh = "syr_s9_26_poll_respond_refresh";
    let identity = alice();
    let room_id = "!s926SecretRoom:example.org";
    let poll_event_id = "$s926SecretEvent:example.org";
    let answer = "s926SecretAnswer";
    let answers = vec![answer.to_owned()];
    let invalid_room = "s926-not-a-room-id";
    let invalid_event = "s926-not-an-event-id";
    let invalid_answers = vec!["".to_owned()];
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("poll-respond-no-start");
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

    let missing_room = respond_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        poll_event_id.to_owned(),
        answers.clone(),
    );
    let invalid_room_respond = respond_plain(
        &rt,
        &shared,
        invalid_room.to_owned(),
        poll_event_id.to_owned(),
        answers.clone(),
    );
    let invalid_event_respond = respond_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        invalid_event.to_owned(),
        answers.clone(),
    );
    let invalid_answers_respond = respond_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        poll_event_id.to_owned(),
        invalid_answers,
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_text = missing_room
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted respond must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_respond
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted respond must fail on invalid room id without a live server");
    let invalid_event_text = invalid_event_respond
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted respond must fail on invalid event id without a live server");
    let invalid_answers_text = invalid_answers_respond
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted respond must fail on invalid answer ids without a live server");

    assert!(
        missing_room_text.contains("v-send.3-poll-room-not-found"),
        "respond must return the registered room-not-found diagnostic: {missing_room_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "respond must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_text.contains("v-send.3-poll-invalid-event-id"),
        "respond must return the registered invalid-event diagnostic: {invalid_event_text}"
    );
    assert!(
        invalid_answers_text.contains("v-send.3-poll-invalid-answer-ids"),
        "respond must return the registered invalid-answer diagnostic: {invalid_answers_text}"
    );
    for (label, text) in [
        ("missing_room", &missing_room_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_event", &invalid_event_text),
        ("invalid_answers", &invalid_answers_text),
    ] {
        assert!(
            !text.contains("p4-s9-26-poll-respond-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text =
        format!("{missing_room_text}{invalid_room_text}{invalid_event_text}{invalid_answers_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(poll_event_id));
    assert!(!text.contains(answer));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
}
