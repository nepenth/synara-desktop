//! P4-S9-6: typed SharedCore consume of the three registered m.direct commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Snapshot DTOs may return user/room ids. Failed errors stay static.
//! Directory visibility and leftover secret envelopes stay off.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-6-it-{tag}-{nanos}"));
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
fn mdirect_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("mdirect_snapshot"));
    assert!(udl.contains("mdirect_add"));
    assert!(udl.contains("mdirect_remove"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("mdirect_snapshot"));
    assert!(shared_core.contains("mdirect_add"));
    assert!(shared_core.contains("mdirect_remove"));
    assert!(shared_core.contains("later_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn mdirect_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s96dm:example.org";
    let user_id = "@bob:example.org";
    let snapshot = rt
        .block_on(shared.mdirect_snapshot())
        .expect_err("no attached m.direct owner");
    let add = rt
        .block_on(shared.mdirect_add(room_id.to_owned(), user_id.to_owned()))
        .expect_err("no attached m.direct owner");
    let remove = rt
        .block_on(shared.mdirect_remove(room_id.to_owned()))
        .expect_err("no attached m.direct owner");
    let snapshot_text = format!("{snapshot:?}{snapshot}");
    let add_text = format!("{add:?}{add}");
    let remove_text = format!("{remove:?}{remove}");
    assert!(snapshot_text.contains("p2-mdirect-snapshot-no-session"));
    assert!(add_text.contains("p2-mdirect-add-no-session"));
    assert!(remove_text.contains("p2-mdirect-remove-no-session"));
    let text = format!("{snapshot_text}{add_text}{remove_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(user_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn mdirect_add_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let user_id = "@bob:example.org";
    let error = test_runtime()
        .block_on(shared.mdirect_add(room_id.clone(), user_id.to_owned()))
        .expect_err("oversize m.direct payload must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s9-6-mdirect-failed"));
    assert!(!text.contains(&room_id));
    assert!(!text.contains(user_id));
}

#[test]
fn mdirect_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_6_mdirect_access";
    let refresh = "syr_s9_6_mdirect_refresh";
    let identity = alice();
    let room_id = "!s96dm:example.org";
    let user_id = "@bob:example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("mdirect-no-start");
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

    let snapshot = rt.block_on(shared.mdirect_snapshot());
    let add = rt.block_on(shared.mdirect_add(room_id.to_owned(), user_id.to_owned()));
    let remove = rt.block_on(shared.mdirect_remove(room_id.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let snapshot_text = match &snapshot {
        Ok(value) => format!("ok:{}:{}", value.room_ids.len(), value.user_ids.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let add_text = add
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered m.direct add handler");
    let remove_text = remove
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered m.direct remove handler");

    assert!(
        snapshot.is_ok() || snapshot_text.contains("v-rooms.5-mdirect-"),
        "snapshot must return the registered handler result: {snapshot_text}"
    );
    assert!(
        !snapshot_text.contains("p4-s9-6-mdirect-failed"),
        "snapshot must not hide a wrong envelope behind the generic fallback: {snapshot_text}"
    );
    assert!(
        add_text.contains("v-rooms.5-mdirect-"),
        "add must return a registered owner diagnostic: {add_text}"
    );
    assert!(
        remove_text.contains("v-rooms.5-mdirect-"),
        "remove must return a registered owner diagnostic: {remove_text}"
    );
    let text = format!("{snapshot_text}{add_text}{remove_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!add_text.contains(room_id));
    assert!(!add_text.contains(user_id));
    assert!(!remove_text.contains(room_id));
    assert!(!remove_text.contains(user_id));
    if snapshot.is_err() {
        assert!(!snapshot_text.contains(room_id));
        assert!(!snapshot_text.contains(user_id));
    }
}
