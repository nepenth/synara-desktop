//! P4-S8: typed SharedCore consume of `matrix_verification_list` only.
//!
//! Calls the already-registered Core handler. Does not start SyncService.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s8-it-{tag}-{nanos}"));
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
fn verification_list_surface_exposes_only_the_registered_list_command() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("verification_list"));
    assert!(!udl.contains("matrix_verification_start"));
    assert!(!udl.contains("matrix_verification_accept"));
    assert!(!udl.contains("matrix_verification_begin_sas"));
    assert!(!udl.contains("matrix_verification_confirm"));
    assert!(!udl.contains("matrix_verification_mismatch"));
    assert!(!udl.contains("matrix_verification_cancel"));
    assert!(!udl.contains("matrix_verification_dismiss"));
    assert!(!udl.contains("matrix_login_password"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("verification_list"));
    assert!(shared_core.contains("typing_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("device_delete_password"));
    assert!(!shared_core.contains("room_leave"));
    assert!(!shared_core.contains("crypto_status"));
}

#[test]
fn verification_list_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let error = test_runtime()
        .block_on(shared.verification_list())
        .expect_err("no attached verification owner");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p2-verification-list-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
}

#[test]
fn verification_list_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s8_verification_access";
    let refresh = "syr_s8_verification_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("verification-list-no-start");
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
    let dto = rt
        .block_on(shared.verification_list())
        .expect("unstarted sync yields the registered handler result");
    assert_eq!(dto.session_generation, 1);
    assert!(dto.requests.is_empty());
    let text = format!("{dto:?}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
