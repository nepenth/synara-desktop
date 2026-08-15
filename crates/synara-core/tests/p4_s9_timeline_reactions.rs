//! P4-S9-20: typed SharedCore consume of the three registered timeline
//! reaction commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo room id, event id,
//! reaction event id, or key. Composer reply draft stays off.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-20-it-{tag}-{nanos}"));
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
fn timeline_reactions_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("reaction_ensure("));
    assert!(udl.contains("reaction_redact("));
    assert!(udl.contains("timeline_reaction_toggle("));
    assert!(udl.contains("dictionary TimelineReactionMutationDto"));
    assert!(udl.contains("dictionary TimelineReactionDto"));
    assert!(udl.contains("interface TimelineReactionError"));
    assert!(udl.contains("timeline_event_readback("));
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
    assert!(shared_core.contains("reaction_ensure("));
    assert!(shared_core.contains("reaction_redact("));
    assert!(shared_core.contains("timeline_reaction_toggle("));
    assert!(shared_core.contains("timeline_event_readback("));
    assert!(shared_core.contains("invites_accept("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn timeline_reactions_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s920SecretRoom:example.org";
    let event_id = "$s920SecretEvent";
    let reaction_event_id = "$s920SecretReaction";
    let key = "s920SecretKey";
    let ensure = rt
        .block_on(shared.reaction_ensure(room_id.to_owned(), event_id.to_owned(), key.to_owned()))
        .expect_err("no attached reaction-ensure owner");
    let redact = rt
        .block_on(shared.reaction_redact(
            room_id.to_owned(),
            event_id.to_owned(),
            reaction_event_id.to_owned(),
            key.to_owned(),
        ))
        .expect_err("no attached reaction-redact owner");
    let toggle = rt
        .block_on(shared.timeline_reaction_toggle(
            room_id.to_owned(),
            event_id.to_owned(),
            key.to_owned(),
        ))
        .expect_err("no attached reaction-toggle owner");
    let ensure_text = format!("{ensure:?}{ensure}");
    let redact_text = format!("{redact:?}{redact}");
    let toggle_text = format!("{toggle:?}{toggle}");
    assert!(ensure_text.contains("p2-reaction-ensure-no-session"));
    assert!(redact_text.contains("p2-reaction-redact-no-session"));
    assert!(toggle_text.contains("p2-timeline-reaction-toggle-no-session"));
    let text = format!("{ensure_text}{redact_text}{toggle_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(reaction_event_id));
    assert!(!text.contains(key));
    assert!(!text.contains("@alice"));
}

#[test]
fn timeline_reactions_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let event_id = "$s920OversizeEvent";
    let reaction_event_id = "$s920OversizeReaction";
    let key = "s920OversizeKey";
    let rt = test_runtime();
    let ensure = rt
        .block_on(shared.reaction_ensure(room_id.clone(), event_id.to_owned(), key.to_owned()))
        .expect_err("oversize ensure payload must fail closed");
    let redact = rt
        .block_on(shared.reaction_redact(
            room_id.clone(),
            event_id.to_owned(),
            reaction_event_id.to_owned(),
            key.to_owned(),
        ))
        .expect_err("oversize redact payload must fail closed");
    let toggle = rt
        .block_on(shared.timeline_reaction_toggle(
            room_id.clone(),
            event_id.to_owned(),
            key.to_owned(),
        ))
        .expect_err("oversize toggle payload must fail closed");
    let ensure_text = format!("{ensure:?}{ensure}");
    let redact_text = format!("{redact:?}{redact}");
    let toggle_text = format!("{toggle:?}{toggle}");
    assert!(ensure_text.contains("p4-s9-20-timeline-reactions-failed"));
    assert!(redact_text.contains("p4-s9-20-timeline-reactions-failed"));
    assert!(toggle_text.contains("p4-s9-20-timeline-reactions-failed"));
    assert!(!ensure_text.contains(&room_id));
    assert!(!redact_text.contains(&room_id));
    assert!(!toggle_text.contains(&room_id));
    assert!(!ensure_text.contains(event_id));
    assert!(!redact_text.contains(reaction_event_id));
    assert!(!toggle_text.contains(key));
}

#[test]
fn timeline_reactions_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_20_timeline_reactions_access";
    let refresh = "syr_s9_20_timeline_reactions_refresh";
    let identity = alice();
    let room_id = "!s920SecretRoom:example.org";
    let event_id = "$s920SecretEvent";
    let reaction_event_id = "$s920SecretReaction";
    let key = "s920SecretKey";
    let invalid_room = "s920-not-a-room-id";
    let invalid_event = "s920-not-an-event-id";
    let invalid_key = "";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-reactions-no-start");
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

    let missing_room_ensure = rt.block_on(shared.reaction_ensure(
        room_id.to_owned(),
        event_id.to_owned(),
        key.to_owned(),
    ));
    let missing_room_redact = rt.block_on(shared.reaction_redact(
        room_id.to_owned(),
        event_id.to_owned(),
        reaction_event_id.to_owned(),
        key.to_owned(),
    ));
    let missing_room_toggle = rt.block_on(shared.timeline_reaction_toggle(
        room_id.to_owned(),
        event_id.to_owned(),
        key.to_owned(),
    ));
    let invalid_room_ensure = rt.block_on(shared.reaction_ensure(
        invalid_room.to_owned(),
        event_id.to_owned(),
        key.to_owned(),
    ));
    let invalid_event_ensure = rt.block_on(shared.reaction_ensure(
        room_id.to_owned(),
        invalid_event.to_owned(),
        key.to_owned(),
    ));
    let invalid_key_ensure = rt.block_on(shared.reaction_ensure(
        room_id.to_owned(),
        event_id.to_owned(),
        invalid_key.to_owned(),
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_ensure_text = missing_room_ensure
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted ensure must fail on local room lookup without a live server");
    let missing_room_redact_text = missing_room_redact
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted redact must fail on local room lookup without a live server");
    let missing_room_toggle_text = missing_room_toggle
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted toggle must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_ensure
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted ensure must fail on invalid room id without a live server");
    let invalid_event_text = invalid_event_ensure
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted ensure must fail on invalid event id without a live server");
    let invalid_key_text = invalid_key_ensure
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted ensure must fail on invalid key without a live server");

    for (label, text) in [
        ("missing_room_ensure", &missing_room_ensure_text),
        ("missing_room_redact", &missing_room_redact_text),
        ("missing_room_toggle", &missing_room_toggle_text),
    ] {
        assert!(
            text.contains("d0.3-timeline-room-not-found"),
            "{label} must return the registered room-not-found diagnostic: {text}"
        );
        assert!(
            !text.contains("p4-s9-20-timeline-reactions-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    assert!(
        invalid_room_text.contains("d0.3-timeline-invalid-room-id"),
        "ensure must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_event_text.contains("v-crypto.6-invalid-event-id"),
        "ensure must return the registered invalid-event diagnostic: {invalid_event_text}"
    );
    assert!(
        invalid_key_text.contains("v-send.2-reaction-invalid-key"),
        "ensure must return the registered invalid-key diagnostic: {invalid_key_text}"
    );
    for (label, text) in [
        ("invalid_room", &invalid_room_text),
        ("invalid_event", &invalid_event_text),
        ("invalid_key", &invalid_key_text),
    ] {
        assert!(
            !text.contains("p4-s9-20-timeline-reactions-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!(
        "{missing_room_ensure_text}{missing_room_redact_text}{missing_room_toggle_text}{invalid_room_text}{invalid_event_text}{invalid_key_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(reaction_event_id));
    assert!(!text.contains(key));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
}
