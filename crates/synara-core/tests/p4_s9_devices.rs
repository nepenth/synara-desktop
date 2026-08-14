//! P4-S9-2: typed SharedCore consume of device snapshot/rename/delete only.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Backup status, room-key transfer status, and cross-signing setup stay off
//! this slice because they sit next to leftover passphrase/path/password
//! envelopes.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-2-it-{tag}-{nanos}"));
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
fn device_surface_exposes_only_the_registered_device_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("device_snapshot"));
    assert!(udl.contains("device_rename"));
    assert!(udl.contains("device_delete_start"));
    assert!(udl.contains("device_delete_cancel"));
    assert!(!udl.contains("device_delete_password"));
    assert!(!udl.contains("matrix_backup_status"));
    assert!(!udl.contains("matrix_room_key_transfer_status"));
    assert!(!udl.contains("matrix_cross_signing_setup"));
    assert!(!udl.contains("backup_setup"));
    assert!(!udl.contains("crypto_status"));
    assert!(!udl.contains("matrix_login_password"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("device_snapshot"));
    assert!(shared_core.contains("device_rename"));
    assert!(shared_core.contains("device_delete_start"));
    assert!(shared_core.contains("device_delete_cancel"));
    assert!(shared_core.contains("verification_start"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("device_delete_password"));
    assert!(!shared_core.contains("backup_status"));
    assert!(!shared_core.contains("room_key_transfer_status"));
    assert!(!shared_core.contains("cross_signing_setup"));
    assert!(!shared_core.contains("mdirect_snapshot"));
    assert!(!shared_core.contains("crypto_status"));
}

#[test]
fn device_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let device_id = "DEVICE_S9_2_BOB";
    let display_name = "Bob Phone";
    let snapshot = rt
        .block_on(shared.device_snapshot())
        .expect_err("no attached device owner");
    let rename = rt
        .block_on(shared.device_rename(device_id.to_owned(), display_name.to_owned()))
        .expect_err("no attached device owner");
    let delete_start = rt
        .block_on(shared.device_delete_start(vec![device_id.to_owned()]))
        .expect_err("no attached device owner");
    let delete_cancel = rt
        .block_on(shared.device_delete_cancel(9, 1))
        .expect_err("no attached device owner");
    let snapshot_text = format!("{snapshot:?}{snapshot}");
    let rename_text = format!("{rename:?}{rename}");
    let delete_start_text = format!("{delete_start:?}{delete_start}");
    let delete_cancel_text = format!("{delete_cancel:?}{delete_cancel}");
    assert!(snapshot_text.contains("p2-device-snapshot-no-session"));
    assert!(rename_text.contains("p2-device-rename-no-session"));
    assert!(delete_start_text.contains("p2-device-delete-start-no-session"));
    assert!(delete_cancel_text.contains("p2-device-delete-cancel-no-session"));
    let text = format!("{snapshot_text}{rename_text}{delete_start_text}{delete_cancel_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(device_id));
    assert!(!text.contains(display_name));
}

#[test]
fn device_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_2_device_access";
    let refresh = "syr_s9_2_device_refresh";
    let identity = alice();
    let device_id = "DEVICEABC";
    let display_name = "Bob Phone";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("devices-no-start");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        device_id.to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist");
    rt.block_on(shared.attach_session_owners())
        .expect("owners attached");

    let snapshot = rt
        .block_on(shared.device_snapshot())
        .expect_err("unstarted sync still uses the registered snapshot handler");
    let rename = rt
        .block_on(shared.device_rename(device_id.to_owned(), display_name.to_owned()))
        .expect_err("unstarted sync still uses the registered rename handler");
    let delete_start = rt
        .block_on(shared.device_delete_start(vec![device_id.to_owned()]))
        .expect_err("unstarted sync still uses the registered delete-start handler");
    let delete_cancel = rt
        .block_on(shared.device_delete_cancel(9, 1))
        .expect_err("unstarted sync still uses the registered delete-cancel handler");
    let snapshot_text = format!("{snapshot:?}{snapshot}");
    let rename_text = format!("{rename:?}{rename}");
    let delete_start_text = format!("{delete_start:?}{delete_start}");
    let delete_cancel_text = format!("{delete_cancel:?}{delete_cancel}");
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    assert!(
        snapshot_text.contains("v-crypto.7-device-snapshot-server-failed"),
        "snapshot must return the registered owner diagnostic: {snapshot_text}"
    );
    assert!(
        !snapshot_text.contains("p4-s9-2-device-failed"),
        "snapshot must not hide a wrong envelope behind the generic fallback: {snapshot_text}"
    );
    assert!(
        rename_text.contains("v-crypto.7-device-rename-failed"),
        "rename must return the registered owner diagnostic: {rename_text}"
    );
    assert!(
        delete_start_text.contains("v-crypto.7-"),
        "delete_start must return a registered owner diagnostic: {delete_start_text}"
    );
    assert!(
        !delete_start_text.contains("p4-s9-2-device-failed"),
        "delete_start must not hide a wrong envelope behind the generic fallback: {delete_start_text}"
    );
    assert!(
        delete_cancel_text.contains("v-crypto.7-device-delete-not-pending"),
        "delete_cancel must return the registered owner diagnostic: {delete_cancel_text}"
    );
    let text = format!("{snapshot_text}{rename_text}{delete_start_text}{delete_cancel_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(device_id));
    assert!(!text.contains(display_name));
}
