//! P4-S9-7: typed SharedCore consume of the five registered room-notes commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Note body text may cross in snapshot/item DTOs. Failed errors stay static
//! and must not echo note body, room id, or item id.
//! Directory visibility and leftover secret envelopes stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, RoomNoteItemDto, SharedCore};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-7-it-{tag}-{nanos}"));
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

fn sample_item() -> RoomNoteItemDto {
    RoomNoteItemDto {
        id: "note-s97".to_owned(),
        kind: "note".to_owned(),
        room_id: "!s97notes:example.org".to_owned(),
        created_at: 1_700_000_000_000.0,
        updated_at: 1_700_000_000_000.0,
        body: Some("secret note body text".to_owned()),
        completed_at: None,
        order: None,
        event_id: None,
        event_ts: None,
        sender: None,
    }
}

#[test]
fn room_notes_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_notes_snapshot"));
    assert!(udl.contains("room_notes_upsert"));
    assert!(udl.contains("room_notes_delete"));
    assert!(udl.contains("room_notes_complete_todo"));
    assert!(udl.contains("room_notes_move_todo"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_notes_snapshot"));
    assert!(shared_core.contains("room_notes_upsert"));
    assert!(shared_core.contains("room_notes_delete"));
    assert!(shared_core.contains("room_notes_complete_todo"));
    assert!(shared_core.contains("room_notes_move_todo"));
    assert!(shared_core.contains("mdirect_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("room_create"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_notes_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let item = sample_item();
    let item_id = item.id.clone();
    let room_id = item.room_id.clone();
    let body = item.body.clone().expect("sample body");
    let snapshot = rt
        .block_on(shared.room_notes_snapshot())
        .expect_err("no attached room-notes owner");
    let upsert = rt
        .block_on(shared.room_notes_upsert(item))
        .expect_err("no attached room-notes owner");
    let delete = rt
        .block_on(shared.room_notes_delete(room_id.clone(), item_id.clone()))
        .expect_err("no attached room-notes owner");
    let complete = rt
        .block_on(shared.room_notes_complete_todo(room_id.clone(), item_id.clone(), true))
        .expect_err("no attached room-notes owner");
    let moved = rt
        .block_on(shared.room_notes_move_todo(room_id.clone(), item_id.clone(), "up".to_owned()))
        .expect_err("no attached room-notes owner");
    let snapshot_text = format!("{snapshot:?}{snapshot}");
    let upsert_text = format!("{upsert:?}{upsert}");
    let delete_text = format!("{delete:?}{delete}");
    let complete_text = format!("{complete:?}{complete}");
    let move_text = format!("{moved:?}{moved}");
    assert!(snapshot_text.contains("p2-room-notes-snapshot-no-session"));
    assert!(upsert_text.contains("p2-room-notes-upsert-no-session"));
    assert!(delete_text.contains("p2-room-notes-delete-no-session"));
    assert!(complete_text.contains("p2-room-notes-complete-todo-no-session"));
    assert!(move_text.contains("p2-room-notes-move-todo-no-session"));
    let text = format!("{snapshot_text}{upsert_text}{delete_text}{complete_text}{move_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(&item_id));
    assert!(!text.contains(&room_id));
    assert!(!text.contains(&body));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_notes_upsert_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let body = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let item = RoomNoteItemDto {
        id: "note-s97-oversize".to_owned(),
        kind: "note".to_owned(),
        room_id: "!s97oversize:example.org".to_owned(),
        created_at: 1_700_000_000_000.0,
        updated_at: 1_700_000_000_000.0,
        body: Some(body.clone()),
        completed_at: None,
        order: None,
        event_id: None,
        event_ts: None,
        sender: None,
    };
    let error = test_runtime()
        .block_on(shared.room_notes_upsert(item))
        .expect_err("oversize room-notes payload must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s9-7-room-notes-failed"));
    assert!(!text.contains(&body));
    assert!(!text.contains("note-s97-oversize"));
    assert!(!text.contains("!s97oversize:example.org"));
}

#[test]
fn room_notes_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_7_room_notes_access";
    let refresh = "syr_s9_7_room_notes_refresh";
    let identity = alice();
    let item = sample_item();
    let item_id = item.id.clone();
    let room_id = item.room_id.clone();
    let body = item.body.clone().expect("sample body");
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-notes-no-start");
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

    let snapshot = rt.block_on(shared.room_notes_snapshot());
    let upsert = rt.block_on(shared.room_notes_upsert(item));
    let delete = rt.block_on(shared.room_notes_delete(room_id.clone(), item_id.clone()));
    let complete =
        rt.block_on(shared.room_notes_complete_todo(room_id.clone(), item_id.clone(), true));
    let moved =
        rt.block_on(shared.room_notes_move_todo(room_id.clone(), item_id.clone(), "up".to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let snapshot_text = match &snapshot {
        Ok(value) => format!("ok:{}", value.items.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let upsert_text = upsert
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-notes upsert handler");
    let delete_text = delete
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-notes delete handler");
    let complete_text = complete
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-notes complete handler");
    let move_text = moved
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-notes move handler");

    assert!(
        snapshot.is_ok() || snapshot_text.contains("v-timeline-room-notes-"),
        "snapshot must return the registered handler result: {snapshot_text}"
    );
    assert!(
        !snapshot_text.contains("p4-s9-7-room-notes-failed"),
        "snapshot must not hide a wrong envelope behind the generic fallback: {snapshot_text}"
    );
    assert!(
        upsert_text.contains("v-timeline-room-notes-"),
        "upsert must return a registered owner diagnostic: {upsert_text}"
    );
    assert!(
        delete_text.contains("v-timeline-room-notes-"),
        "delete must return a registered owner diagnostic: {delete_text}"
    );
    assert!(
        complete_text.contains("v-timeline-room-notes-"),
        "complete must return a registered owner diagnostic: {complete_text}"
    );
    assert!(
        move_text.contains("v-timeline-room-notes-"),
        "move must return a registered owner diagnostic: {move_text}"
    );
    let text = format!("{snapshot_text}{upsert_text}{delete_text}{complete_text}{move_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(&item_id));
    assert!(!text.contains(&room_id));
    assert!(!text.contains(&body));
}
