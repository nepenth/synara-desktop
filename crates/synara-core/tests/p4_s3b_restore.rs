//! P4-S3b: restore an already-persisted session from the S3a vault.
//!
//! Integration tests compile the published lib (not crate-internal `#[cfg(test)]`
//! modules). This is not `matrix_restore_session` and does not use a password.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::lifecycle::{SessionMaterial, SessionMaterialId};
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
    let root = std::env::temp_dir().join(format!("synara-p4-s3b-it-{tag}-{nanos}"));
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
fn restore_without_vault_fails_closed_without_echoing_identity() {
    let shared = SharedCore::new();
    let root = temp_root("no-vault");
    let error = test_runtime()
        .block_on(shared.restore_persisted_session(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("fail-closed vault cannot restore");
    let text = format!("{error:?}");
    assert!(
        text.contains("p4-s3b-secret-vault-unavailable")
            || text.contains("p4-s3b-session-material-missing")
    );
    assert!(!text.contains("@alice"));
    assert!(!text.contains("matrix.example.org"));
    assert!(!text.contains(root.to_string_lossy().as_ref()));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn restore_rejects_hostile_identity_without_echo() {
    let store = Box::new(MemoryCallbackVault(Arc::new(Mutex::new(HashMap::new()))));
    let shared = SharedCore::new_with_secret_store(store);
    let root = temp_root("hostile");
    let hostile = "https://user:secret@evil.example/?password=hunter2";
    let error = test_runtime()
        .block_on(shared.restore_persisted_session(
            "not-a-user".to_owned(),
            hostile.to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("invalid identity");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s3b-identity-invalid"));
    assert!(!text.contains("secret"));
    assert!(!text.contains("hunter2"));
    assert!(!text.contains("evil.example"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn restore_rejects_relative_store_root_without_echo() {
    let store = Box::new(MemoryCallbackVault(Arc::new(Mutex::new(HashMap::new()))));
    let shared = SharedCore::new_with_secret_store(store);
    let error = test_runtime()
        .block_on(shared.restore_persisted_session(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            "../not-absolute".to_owned(),
        ))
        .expect_err("relative store root");
    let text = format!("{error:?}");
    assert!(text.contains("p4-s3b-store-root-invalid"));
    assert!(!text.contains("not-absolute"));
    assert!(!text.contains("@alice"));
}

#[test]
fn restore_from_vault_installs_session_without_password_or_token_leak() {
    let access = "syt_s3b_access_token_value";
    let refresh = "syr_s3b_refresh_token_value";
    let identity = alice();
    let material =
        SessionMaterial::from_matrix_tokens(&identity, "DEVICEABC", access, Some(refresh)).unwrap();
    let map = Arc::new(Mutex::new(HashMap::new()));
    map.lock().expect("vault").insert(
        SessionMaterialId::from_identity(&identity)
            .account()
            .to_owned(),
        material.as_bytes().to_vec(),
    );
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(map)));
    let root = temp_root("restore");
    let rt = test_runtime();
    let _enter = rt.enter();
    let dto = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect("restore");
    assert_eq!(dto.user_id, "@alice:example.org");
    assert_eq!(dto.device_id, "DEVICEABC");
    assert_eq!(dto.homeserver_url, "https://matrix.example.org");
    let dbg = format!("{dto:?}");
    assert!(!dbg.contains(access));
    assert!(!dbg.contains(refresh));
    assert!(!dbg.contains("password"));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
