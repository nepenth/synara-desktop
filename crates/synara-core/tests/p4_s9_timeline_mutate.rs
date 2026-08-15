//! P4-S9-27: typed SharedCore consume of the registered timeline
//! edit / redact / report commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo body, event id, room id,
//! or reason. Pin/unpin stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, SharedCore, TimelineMutateDto, TimelineMutateError,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-27-it-{tag}-{nanos}"));
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
) -> Result<TimelineMutateDto, TimelineMutateError> {
    rt.block_on(shared.timeline_edit_text(room_id, event_id, body, None))
}

fn redact_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
    reason: Option<String>,
) -> Result<TimelineMutateDto, TimelineMutateError> {
    rt.block_on(shared.timeline_redact(room_id, event_id, reason))
}

fn report_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
    reason: Option<String>,
) -> Result<TimelineMutateDto, TimelineMutateError> {
    rt.block_on(shared.timeline_report(room_id, event_id, reason))
}

fn error_text(error: &TimelineMutateError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn timeline_mutate_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_edit_text("));
    assert!(udl.contains("timeline_redact("));
    assert!(udl.contains("timeline_report("));
    assert!(udl.contains("dictionary TimelineMutateDto"));
    assert!(udl.contains("interface TimelineMutateError"));
    assert!(udl.contains("poll_respond("));
    assert!(udl.contains("edit_message("));
    assert!(udl.contains("send_poll("));
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
    assert!(shared_core.contains("timeline_edit_text("));
    assert!(shared_core.contains("timeline_redact("));
    assert!(shared_core.contains("timeline_report("));
    assert!(shared_core.contains("poll_respond("));
    assert!(shared_core.contains("edit_message("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));
    assert!(!shared_core.contains("cross_signing_status"));
    assert!(!shared_core.contains("cross_signing_setup"));
    assert!(!shared_core.contains("room_key_transfer_status"));
}

#[test]
fn timeline_mutate_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s927SecretRoom:example.org";
    let event_id = "$s927SecretEvent:example.org";
    let body = "s927SecretBody";
    let reason = "s927SecretReason";
    let edit = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        body.to_owned(),
    )
    .expect_err("no attached timeline-edit-text owner");
    let redact = redact_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        Some(reason.to_owned()),
    )
    .expect_err("no attached timeline-redact owner");
    let report = report_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        Some(reason.to_owned()),
    )
    .expect_err("no attached timeline-report owner");
    let edit_text = error_text(&edit);
    let redact_text = error_text(&redact);
    let report_text = error_text(&report);
    assert!(edit_text.contains("p2-timeline-edit-text-no-session"));
    assert!(redact_text.contains("p2-timeline-redact-no-session"));
    assert!(report_text.contains("p2-timeline-report-no-session"));
    let text = format!("{edit_text}{redact_text}{report_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(body));
    assert!(!text.contains(reason));
    assert!(!text.contains("@alice"));
}

#[test]
fn timeline_mutate_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s927OversizeRoom:example.org";
    let event_id = "$s927OversizeEvent:example.org";
    let body = format!(
        "s927OversizeBody{}",
        "b".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let reason = format!(
        "s927OversizeReason{}",
        "r".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let edit = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        body.clone(),
    )
    .expect_err("oversize edit payload must fail closed");
    let redact = redact_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        Some(reason.clone()),
    )
    .expect_err("oversize redact payload must fail closed");
    let report = report_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        Some(reason.clone()),
    )
    .expect_err("oversize report payload must fail closed");
    let edit_text = error_text(&edit);
    let redact_text = error_text(&redact);
    let report_text = error_text(&report);
    assert!(edit_text.contains("p4-s9-27-timeline-mutate-failed"));
    assert!(redact_text.contains("p4-s9-27-timeline-mutate-failed"));
    assert!(report_text.contains("p4-s9-27-timeline-mutate-failed"));
    let text = format!("{edit_text}{redact_text}{report_text}");
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(&body));
    assert!(!text.contains(&reason));
    assert!(!text.contains("s927SecretBody"));
    assert!(!text.contains("s927SecretReason"));
}

#[test]
fn timeline_mutate_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_27_timeline_mutate_access";
    let refresh = "syr_s9_27_timeline_mutate_refresh";
    let identity = alice();
    let room_id = "!s927SecretRoom:example.org";
    let event_id = "$s927SecretEvent:example.org";
    let body = "s927SecretBody";
    let reason = "s927SecretReason";
    let invalid_room = "s927-not-a-room-id";
    let invalid_event = "s927-not-an-event-id";
    let empty_body = "";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-mutate-no-start");
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

    let missing_room_edit = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        body.to_owned(),
    );
    let missing_room_redact = redact_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        Some(reason.to_owned()),
    );
    let missing_room_report = report_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        Some(reason.to_owned()),
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
    let empty_body_edit = edit_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        event_id.to_owned(),
        empty_body.to_owned(),
    );
    let invalid_event_redact = redact_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        invalid_event.to_owned(),
        Some(reason.to_owned()),
    );
    let invalid_event_report = report_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        invalid_event.to_owned(),
        Some(reason.to_owned()),
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_edit_text = missing_room_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on local room lookup without a live server");
    let missing_room_redact_text = missing_room_redact
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted redact must fail on local room lookup without a live server");
    let missing_room_report_text = missing_room_report
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted report must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on invalid room id without a live server");
    let invalid_event_edit_text = invalid_event_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on invalid event id without a live server");
    let empty_body_text = empty_body_edit
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted edit must fail on empty body without a live server");
    let invalid_event_redact_text = invalid_event_redact
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted redact must fail on invalid event id without a live server");
    let invalid_event_report_text = invalid_event_report
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted report must fail on invalid event id without a live server");

    assert!(
        missing_room_edit_text.contains("v-timeline-edit-room-not-found"),
        "edit must return the registered room-not-found diagnostic: {missing_room_edit_text}"
    );
    assert!(
        missing_room_redact_text.contains("v-timeline-redact-room-not-found"),
        "redact must return the registered room-not-found diagnostic: {missing_room_redact_text}"
    );
    assert!(
        missing_room_report_text.contains("v-timeline-report-room-not-found"),
        "report must return the registered room-not-found diagnostic: {missing_room_report_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "edit must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_edit_text.contains("v-timeline-edit-invalid-event-id"),
        "edit must return the registered invalid-event diagnostic: {invalid_event_edit_text}"
    );
    assert!(
        empty_body_text.contains("v-timeline-edit-empty-body"),
        "edit must return the registered empty-body diagnostic: {empty_body_text}"
    );
    assert!(
        invalid_event_redact_text.contains("v-timeline-redact-invalid-event-id"),
        "redact must return the registered invalid-event diagnostic: {invalid_event_redact_text}"
    );
    assert!(
        invalid_event_report_text.contains("v-timeline-report-invalid-event-id"),
        "report must return the registered invalid-event diagnostic: {invalid_event_report_text}"
    );
    for (label, text) in [
        ("missing_room_edit", &missing_room_edit_text),
        ("missing_room_redact", &missing_room_redact_text),
        ("missing_room_report", &missing_room_report_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_event_edit", &invalid_event_edit_text),
        ("empty_body", &empty_body_text),
        ("invalid_event_redact", &invalid_event_redact_text),
        ("invalid_event_report", &invalid_event_report_text),
    ] {
        assert!(
            !text.contains("p4-s9-27-timeline-mutate-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!(
        "{missing_room_edit_text}{missing_room_redact_text}{missing_room_report_text}{invalid_room_text}{invalid_event_edit_text}{empty_body_text}{invalid_event_redact_text}{invalid_event_report_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(body));
    assert!(!text.contains(reason));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
}
