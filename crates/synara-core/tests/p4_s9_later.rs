//! P4-S9-5: typed SharedCore consume of the six registered later commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Item ids/timestamps may cross. Directory visibility and leftover
//! secret envelopes stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, LaterItemDto, SharedCore};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-5-it-{tag}-{nanos}"));
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

fn sample_item() -> LaterItemDto {
    LaterItemDto {
        id: "later-s95".to_owned(),
        kind: "saved".to_owned(),
        room_id: "!s95later:example.org".to_owned(),
        event_id: "$s95event".to_owned(),
        created_at: 1_700_000_000_000.0,
        due_ts: None,
        reminded_at: None,
        completed_at: None,
    }
}

#[test]
fn later_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("later_snapshot"));
    assert!(udl.contains("later_upsert"));
    assert!(udl.contains("later_complete"));
    assert!(udl.contains("later_snooze"));
    assert!(udl.contains("later_clear_completed"));
    assert!(udl.contains("later_mark_reminded"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("later_snapshot"));
    assert!(shared_core.contains("later_upsert"));
    assert!(shared_core.contains("later_complete"));
    assert!(shared_core.contains("later_snooze"));
    assert!(shared_core.contains("later_clear_completed"));
    assert!(shared_core.contains("later_mark_reminded"));
    assert!(shared_core.contains("get_global_image_packs"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("room_leave"));
    assert!(!shared_core.contains("room_join("));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn later_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let item = sample_item();
    let item_id = item.id.clone();
    let room_id = item.room_id.clone();
    let event_id = item.event_id.clone();
    let snapshot = rt
        .block_on(shared.later_snapshot())
        .expect_err("no attached later owner");
    let upsert = rt
        .block_on(shared.later_upsert(item))
        .expect_err("no attached later owner");
    let complete = rt
        .block_on(shared.later_complete(item_id.clone(), Some(1_700_000_100_000.0)))
        .expect_err("no attached later owner");
    let snooze = rt
        .block_on(shared.later_snooze(item_id.clone(), 1_700_000_200_000.0))
        .expect_err("no attached later owner");
    let clear = rt
        .block_on(shared.later_clear_completed())
        .expect_err("no attached later owner");
    let reminded = rt
        .block_on(shared.later_mark_reminded(item_id.clone(), Some(1_700_000_300_000.0)))
        .expect_err("no attached later owner");
    let snapshot_text = format!("{snapshot:?}{snapshot}");
    let upsert_text = format!("{upsert:?}{upsert}");
    let complete_text = format!("{complete:?}{complete}");
    let snooze_text = format!("{snooze:?}{snooze}");
    let clear_text = format!("{clear:?}{clear}");
    let reminded_text = format!("{reminded:?}{reminded}");
    assert!(snapshot_text.contains("p2-later-snapshot-no-session"));
    assert!(upsert_text.contains("p2-later-upsert-no-session"));
    assert!(complete_text.contains("p2-later-complete-no-session"));
    assert!(snooze_text.contains("p2-later-snooze-no-session"));
    assert!(clear_text.contains("p2-later-clear-completed-no-session"));
    assert!(reminded_text.contains("p2-later-mark-reminded-no-session"));
    let text = format!(
        "{snapshot_text}{upsert_text}{complete_text}{snooze_text}{clear_text}{reminded_text}"
    );
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(&item_id));
    assert!(!text.contains(&room_id));
    assert!(!text.contains(&event_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn later_upsert_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let item = LaterItemDto {
        id: "later-s95-oversize".to_owned(),
        kind: "saved".to_owned(),
        room_id: room_id.clone(),
        event_id: "$s95oversize".to_owned(),
        created_at: 1_700_000_000_000.0,
        due_ts: None,
        reminded_at: None,
        completed_at: None,
    };
    let error = test_runtime()
        .block_on(shared.later_upsert(item))
        .expect_err("oversize later payload must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s9-5-later-failed"));
    assert!(!text.contains(&room_id));
    assert!(!text.contains("later-s95-oversize"));
    assert!(!text.contains("$s95oversize"));
}

#[test]
fn later_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_5_later_access";
    let refresh = "syr_s9_5_later_refresh";
    let identity = alice();
    let item = sample_item();
    let item_id = item.id.clone();
    let room_id = item.room_id.clone();
    let event_id = item.event_id.clone();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("later-no-start");
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

    let snapshot = rt.block_on(shared.later_snapshot());
    let upsert = rt.block_on(shared.later_upsert(item));
    let complete = rt.block_on(shared.later_complete(item_id.clone(), Some(1_700_000_100_000.0)));
    let snooze = rt.block_on(shared.later_snooze(item_id.clone(), 1_700_000_200_000.0));
    let clear = rt.block_on(shared.later_clear_completed());
    let reminded =
        rt.block_on(shared.later_mark_reminded(item_id.clone(), Some(1_700_000_300_000.0)));
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
        .expect("unstarted sync still uses the registered later upsert handler");
    let complete_text = complete
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered later complete handler");
    let snooze_text = snooze
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered later snooze handler");
    let clear_text = clear
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered later clear handler");
    let reminded_text = reminded
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered later mark-reminded handler");

    assert!(
        snapshot.is_ok() || snapshot_text.contains("v-timeline-later-"),
        "snapshot must return the registered handler result: {snapshot_text}"
    );
    assert!(
        !snapshot_text.contains("p4-s9-5-later-failed"),
        "snapshot must not hide a wrong envelope behind the generic fallback: {snapshot_text}"
    );
    assert!(
        upsert_text.contains("v-timeline-later-"),
        "upsert must return a registered owner diagnostic: {upsert_text}"
    );
    assert!(
        complete_text.contains("v-timeline-later-"),
        "complete must return a registered owner diagnostic: {complete_text}"
    );
    assert!(
        snooze_text.contains("v-timeline-later-"),
        "snooze must return a registered owner diagnostic: {snooze_text}"
    );
    assert!(
        clear_text.contains("v-timeline-later-"),
        "clear must return a registered owner diagnostic: {clear_text}"
    );
    assert!(
        reminded_text.contains("v-timeline-later-"),
        "mark_reminded must return a registered owner diagnostic: {reminded_text}"
    );
    let text = format!(
        "{snapshot_text}{upsert_text}{complete_text}{snooze_text}{clear_text}{reminded_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(&item_id));
    assert!(!text.contains(&room_id));
    assert!(!text.contains(&event_id));
}
