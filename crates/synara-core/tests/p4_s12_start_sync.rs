//! P4-S12: start the already-attached SyncService on SharedCore.
//!
//! This is not Core.command, not leftover registration, and not P4
//! acceptance. NSE still cannot start sync.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s12-it-{tag}-{nanos}"));
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
fn start_sync_surface_is_attached_only_and_not_a_leftover() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary SyncStartDto"));
    assert!(udl.contains("interface SyncStartError"));
    assert!(udl.contains("SyncStartDto start_sync()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("start_sync()"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_login_password"));
}

#[test]
fn start_sync_without_attach_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let error = test_runtime()
        .block_on(shared.start_sync())
        .expect_err("start requires attach");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s12-sync-not-attached"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
}

#[test]
fn start_sync_after_planted_attach_returns_privacy_safe_readiness() {
    let access = "syt_s12_start_sync_access";
    let refresh = "syr_s12_start_sync_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("start-after-attach");
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
    .expect("planted persist retains a Client");
    rt.block_on(shared.attach_session_owners())
        .expect("attach after retained session");
    let dto = rt
        .block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
                .await
                .expect("start_sync timed out")
        })
        .expect("start after attach");
    let dto_text = format!("{dto:?}");
    assert!(!dto.readiness.is_empty());
    assert!(dto.session_generation > 0);
    assert!(dto.offline_mode_enabled);
    assert!(!dto_text.contains(access));
    assert!(!dto_text.contains(refresh));
    assert!(!dto_text.contains("password"));
    assert!(!dto_text.contains("@alice"));
    assert!(!dto_text.contains("https://"));
    let second = rt
        .block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
                .await
                .expect("second start_sync timed out")
        })
        .expect("second start is a restart");
    let second_text = format!("{second:?}");
    assert!(!second_text.contains(access));
    assert!(!second_text.contains(refresh));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn start_sync_on_nse_store_fails_closed_without_echo() {
    let access = "syt_s12_nse_start_access";
    let refresh = "syr_s12_nse_start_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("nse-forbids-start");
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
    rt.block_on(shared.nse_open_read_only_store(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
    ))
    .expect("planted NSE open");
    let error = rt
        .block_on(shared.start_sync())
        .expect_err("NSE cannot start sync");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s12-nse-forbids-start"));
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("@alice"));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
