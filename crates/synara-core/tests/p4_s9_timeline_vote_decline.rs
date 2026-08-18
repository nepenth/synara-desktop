//! P4-S9-29: typed SharedCore consume of the registered timeline
//! poll-vote / call-decline commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo event id, room id, or answer.
//! Session/status reads stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, SharedCore, TimelineVoteDeclineDto,
    TimelineVoteDeclineError,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-29-it-{tag}-{nanos}"));
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

fn vote_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
    answer_ids: Vec<String>,
) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
    rt.block_on(shared.timeline_poll_vote(room_id, event_id, answer_ids))
}

fn decline_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
    rt.block_on(shared.timeline_call_decline(room_id, event_id))
}

fn error_text(error: &TimelineVoteDeclineError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn timeline_vote_decline_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_poll_vote("));
    assert!(udl.contains("timeline_call_decline("));
    assert!(udl.contains("dictionary TimelineVoteDeclineDto"));
    assert!(udl.contains("interface TimelineVoteDeclineError"));
    assert!(udl.contains("timeline_pin("));
    assert!(udl.contains("timeline_unpin("));
    assert!(udl.contains("timeline_edit_text("));
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
    assert!(shared_core.contains("timeline_poll_vote("));
    assert!(shared_core.contains("timeline_call_decline("));
    assert!(shared_core.contains("timeline_pin("));
    assert!(shared_core.contains("timeline_unpin("));
    assert!(shared_core.contains("timeline_edit_text("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));
}

#[test]
fn timeline_vote_decline_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s929SecretRoom:example.org";
    let event_id = "$s929SecretEvent:example.org";
    let answer = "s929SecretAnswer";
    let vote = vote_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        vec![answer.to_owned()],
    )
    .expect_err("no attached timeline-poll-vote owner");
    let decline = decline_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned())
        .expect_err("no attached timeline-call-decline owner");
    let vote_text = error_text(&vote);
    let decline_text = error_text(&decline);
    assert!(vote_text.contains("p2-timeline-poll-vote-no-session"));
    assert!(decline_text.contains("p2-timeline-call-decline-no-session"));
    let text = format!("{vote_text}{decline_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(answer));
    assert!(!text.contains("@alice"));
}

#[test]
fn timeline_vote_decline_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s929OversizeRoom:example.org";
    let event_id = "$s929OversizeEvent:example.org";
    let answer = format!(
        "s929OversizeAnswer{}",
        "a".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let oversize_room = format!(
        "!s929OversizeRoom{}:example.org",
        "r".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let oversize_event = format!(
        "$s929OversizeEvent{}",
        "e".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let vote = vote_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        vec![answer.clone()],
    )
    .expect_err("oversize vote payload must fail closed");
    let decline = decline_plain(&rt, &shared, oversize_room.clone(), oversize_event.clone())
        .expect_err("oversize decline payload must fail closed");
    let vote_text = error_text(&vote);
    let decline_text = error_text(&decline);
    assert!(vote_text.contains("p4-s9-29-timeline-vote-decline-failed"));
    assert!(decline_text.contains("p4-s9-29-timeline-vote-decline-failed"));
    let text = format!("{vote_text}{decline_text}");
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(&answer));
    assert!(!text.contains(&oversize_room));
    assert!(!text.contains(&oversize_event));
    assert!(!text.contains("s929SecretAnswer"));
    assert!(!text.contains("s929SecretRoom"));
    assert!(!text.contains("s929SecretEvent"));
}

#[test]
fn timeline_vote_decline_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_29_timeline_vote_decline_access";
    let refresh = "syr_s9_29_timeline_vote_decline_refresh";
    let identity = alice();
    let room_id = "!s929SecretRoom:example.org";
    let event_id = "$s929SecretEvent:example.org";
    let answer = "s929SecretAnswer";
    let invalid_room = "s929-not-a-room-id";
    let invalid_event = "s929-not-an-event-id";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-vote-decline-no-start");
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

    let missing_room_vote = vote_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        vec![answer.to_owned()],
    );
    let missing_room_decline = decline_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned());
    let invalid_room_vote = vote_plain(
        &rt,
        &shared,
        invalid_room.to_owned(),
        event_id.to_owned(),
        vec![answer.to_owned()],
    );
    let invalid_event_vote = vote_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        invalid_event.to_owned(),
        vec![answer.to_owned()],
    );
    let invalid_event_decline =
        decline_plain(&rt, &shared, room_id.to_owned(), invalid_event.to_owned());
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_vote_text = missing_room_vote
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted vote must fail on local room lookup without a live server");
    let missing_room_decline_text = missing_room_decline
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted decline must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_vote
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted vote must fail on invalid room id without a live server");
    let invalid_event_vote_text = invalid_event_vote
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted vote must fail on invalid event id without a live server");
    let invalid_event_decline_text = invalid_event_decline
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted decline must fail on invalid event id without a live server");

    assert!(
        missing_room_vote_text.contains("v-timeline-poll-vote-room-not-found"),
        "vote must return the registered room-not-found diagnostic: {missing_room_vote_text}"
    );
    assert!(
        missing_room_decline_text.contains("v-timeline-call-decline-room-not-found"),
        "decline must return the registered room-not-found diagnostic: {missing_room_decline_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "vote must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_vote_text.contains("v-timeline-poll-vote-invalid-event-id"),
        "vote must return the registered invalid-event diagnostic: {invalid_event_vote_text}"
    );
    assert!(
        invalid_event_decline_text.contains("v-timeline-call-decline-invalid-event-id"),
        "decline must return the registered invalid-event diagnostic: {invalid_event_decline_text}"
    );
    for (label, text) in [
        ("missing_room_vote", &missing_room_vote_text),
        ("missing_room_decline", &missing_room_decline_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_event_vote", &invalid_event_vote_text),
        ("invalid_event_decline", &invalid_event_decline_text),
    ] {
        assert!(
            !text.contains("p4-s9-29-timeline-vote-decline-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!(
        "{missing_room_vote_text}{missing_room_decline_text}{invalid_room_text}{invalid_event_vote_text}{invalid_event_decline_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(answer));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
}
