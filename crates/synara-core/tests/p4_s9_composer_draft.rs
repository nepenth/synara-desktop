//! P4-S9-21: typed SharedCore consume of the three registered composer
//! reply-draft commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo room id or event id.
//! Poll, edit, and respond stay off.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-21-it-{tag}-{nanos}"));
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
fn composer_reply_draft_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("composer_set_reply_draft("));
    assert!(udl.contains("composer_get_reply_draft("));
    assert!(udl.contains("composer_clear_reply_draft("));
    assert!(udl.contains("dictionary ComposerReplyDraftDto"));
    assert!(udl.contains("dictionary ComposerReplyDraftPreviewDto"));
    assert!(udl.contains("interface ComposerReplyDraftError"));
    assert!(udl.contains("reaction_ensure("));
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
    assert!(shared_core.contains("composer_set_reply_draft("));
    assert!(shared_core.contains("composer_get_reply_draft("));
    assert!(shared_core.contains("composer_clear_reply_draft("));
    assert!(shared_core.contains("reaction_ensure("));
    assert!(shared_core.contains("timeline_event_readback("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn composer_reply_draft_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s921SecretRoom:example.org";
    let event_id = "$s921SecretEvent";
    let set = rt
        .block_on(shared.composer_set_reply_draft(room_id.to_owned(), event_id.to_owned(), false))
        .expect_err("no attached composer-set-reply-draft owner");
    let get = rt
        .block_on(shared.composer_get_reply_draft(room_id.to_owned()))
        .expect_err("no attached composer-get-reply-draft owner");
    let clear = rt
        .block_on(shared.composer_clear_reply_draft(room_id.to_owned()))
        .expect_err("no attached composer-clear-reply-draft owner");
    let set_text = format!("{set:?}{set}");
    let get_text = format!("{get:?}{get}");
    let clear_text = format!("{clear:?}{clear}");
    assert!(set_text.contains("p2-composer-set-reply-draft-no-session"));
    assert!(get_text.contains("p2-composer-get-reply-draft-no-session"));
    assert!(clear_text.contains("p2-composer-clear-reply-draft-no-session"));
    let text = format!("{set_text}{get_text}{clear_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn composer_reply_draft_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let event_id = "$s921OversizeEvent";
    let rt = test_runtime();
    let set = rt
        .block_on(shared.composer_set_reply_draft(room_id.clone(), event_id.to_owned(), false))
        .expect_err("oversize set payload must fail closed");
    let get = rt
        .block_on(shared.composer_get_reply_draft(room_id.clone()))
        .expect_err("oversize get payload must fail closed");
    let clear = rt
        .block_on(shared.composer_clear_reply_draft(room_id.clone()))
        .expect_err("oversize clear payload must fail closed");
    let set_text = format!("{set:?}{set}");
    let get_text = format!("{get:?}{get}");
    let clear_text = format!("{clear:?}{clear}");
    assert!(set_text.contains("p4-s9-21-composer-reply-draft-failed"));
    assert!(get_text.contains("p4-s9-21-composer-reply-draft-failed"));
    assert!(clear_text.contains("p4-s9-21-composer-reply-draft-failed"));
    assert!(!set_text.contains(&room_id));
    assert!(!get_text.contains(&room_id));
    assert!(!clear_text.contains(&room_id));
    assert!(!set_text.contains(event_id));
}

#[test]
fn composer_reply_draft_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_21_composer_reply_draft_access";
    let refresh = "syr_s9_21_composer_reply_draft_refresh";
    let identity = alice();
    let room_id = "!s921SecretRoom:example.org";
    let event_id = "$s921SecretEvent";
    let invalid_room = "s921-not-a-room-id";
    let invalid_event = "s921-not-an-event-id";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("composer-reply-draft-no-start");
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

    let missing_room_set = rt.block_on(shared.composer_set_reply_draft(
        room_id.to_owned(),
        event_id.to_owned(),
        false,
    ));
    let invalid_room_set = rt.block_on(shared.composer_set_reply_draft(
        invalid_room.to_owned(),
        event_id.to_owned(),
        false,
    ));
    let invalid_event_set = rt.block_on(shared.composer_set_reply_draft(
        room_id.to_owned(),
        invalid_event.to_owned(),
        false,
    ));
    let planted_get = rt.block_on(shared.composer_get_reply_draft(room_id.to_owned()));
    let planted_clear = rt.block_on(shared.composer_clear_reply_draft(room_id.to_owned()));
    let invalid_room_get = rt.block_on(shared.composer_get_reply_draft(invalid_room.to_owned()));
    let invalid_room_clear =
        rt.block_on(shared.composer_clear_reply_draft(invalid_room.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_set_text = missing_room_set
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set must fail on local room lookup without a live server");
    let invalid_room_set_text = invalid_room_set
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set must fail on invalid room id without a live server");
    let invalid_event_set_text = invalid_event_set
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set must fail on invalid event id without a live server");
    let planted_get_readback = planted_get
        .as_ref()
        .expect("planted get of a valid room returns the empty readback without a live server");
    let planted_clear_readback = planted_clear
        .as_ref()
        .expect("planted clear of a valid room returns the cleared readback without a live server");
    let invalid_room_get_text = invalid_room_get
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted get must fail on invalid room id without a live server");
    let invalid_room_clear_text = invalid_room_clear
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted clear must fail on invalid room id without a live server");

    assert!(
        missing_room_set_text.contains("v-timeline-reply-draft-room-not-found"),
        "set must return the registered room-not-found diagnostic: {missing_room_set_text}"
    );
    assert!(
        invalid_room_set_text.contains("d0.4-send-invalid-room-id"),
        "set must return the registered invalid-room diagnostic: {invalid_room_set_text}"
    );
    assert!(
        invalid_event_set_text.contains("v-timeline-reply-draft-invalid-event-id"),
        "set must return the registered invalid-event diagnostic: {invalid_event_set_text}"
    );
    assert_eq!(planted_get_readback.status, "empty");
    assert!(planted_get_readback.draft.is_none());
    assert_eq!(planted_clear_readback.status, "cleared");
    assert!(planted_clear_readback.draft.is_none());
    assert!(
        invalid_room_get_text.contains("d0.4-send-invalid-room-id"),
        "get must return the registered invalid-room diagnostic: {invalid_room_get_text}"
    );
    assert!(
        invalid_room_clear_text.contains("d0.4-send-invalid-room-id"),
        "clear must return the registered invalid-room diagnostic: {invalid_room_clear_text}"
    );
    for (label, text) in [
        ("missing_room_set", &missing_room_set_text),
        ("invalid_room_set", &invalid_room_set_text),
        ("invalid_event_set", &invalid_event_set_text),
        ("invalid_room_get", &invalid_room_get_text),
        ("invalid_room_clear", &invalid_room_clear_text),
    ] {
        assert!(
            !text.contains("p4-s9-21-composer-reply-draft-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!(
        "{missing_room_set_text}{invalid_room_set_text}{invalid_event_set_text}{invalid_room_get_text}{invalid_room_clear_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(event_id));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_event));
}
