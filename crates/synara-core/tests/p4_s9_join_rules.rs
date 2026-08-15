//! P4-S9-3: typed SharedCore consume of `matrix_room_join_rule_snapshot` only.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! There is no join-rule writer on Core. Image packs, room leave/join, and
//! leftover secret envelopes stay off this slice.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-3-it-{tag}-{nanos}"));
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
fn join_rule_surface_exposes_only_the_registered_snapshot() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_join_rule_snapshot"));
    assert!(!udl.contains("set_room_join_rule"));
    assert!(!udl.contains("matrix_room_invite"));
    assert!(!udl.contains("matrix_login_password"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_join_rule_snapshot"));
    assert!(shared_core.contains("device_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("set_room_join_rule"));
    assert!(!shared_core.contains("room_invite"));
    assert!(!shared_core.contains("room_kick"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn join_rule_snapshot_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let room_id = "!s93join:example.org";
    let error = test_runtime()
        .block_on(shared.room_join_rule_snapshot(room_id.to_owned(), 1))
        .expect_err("no attached join-rule owner");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p2-join-rule-snapshot-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn join_rule_snapshot_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_3_join_access";
    let refresh = "syr_s9_3_join_refresh";
    let identity = alice();
    let room_id = "!s93join:example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("join-rules-no-start");
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

    let error = rt
        .block_on(shared.room_join_rule_snapshot(room_id.to_owned(), 1))
        .expect_err("unstarted sync still uses the registered snapshot handler");
    let text = format!("{error:?}{error}");
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    assert!(
        text.contains("v-send.r-room-profile-join-rule-room-not-found"),
        "snapshot must return the registered owner diagnostic: {text}"
    );
    assert!(
        !text.contains("p4-s9-3-join-rule-failed"),
        "snapshot must not hide a wrong envelope behind the generic fallback: {text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
}
