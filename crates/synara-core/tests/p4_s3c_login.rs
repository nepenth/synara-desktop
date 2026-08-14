//! P4-S3c: dedicated SharedCore password-login FFI.
//!
//! Password is a method argument only. This is not `matrix_login_password`
//! and does not attach owners or expose `Core.command`.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s3c-it-{tag}-{nanos}"));
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
fn login_surface_does_not_register_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("login_with_password"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("attach_"));
}

#[test]
fn login_without_vault_fails_closed_without_echoing_password() {
    let shared = SharedCore::new();
    let root = temp_root("no-vault");
    let password = "hunter2-s3c-secret";
    let error = test_runtime()
        .block_on(shared.login_with_password(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            root.to_string_lossy().into_owned(),
            password.to_owned(),
        ))
        .expect_err("fail-closed vault cannot login");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s3c-secret-vault-unavailable"));
    assert!(!text.contains(password));
    assert!(!text.contains("hunter2"));
    assert!(!text.contains("@alice"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn login_rejects_hostile_identity_without_echoing_password() {
    let store = Box::new(MemoryCallbackVault(Arc::new(Mutex::new(HashMap::new()))));
    let shared = SharedCore::new_with_secret_store(store);
    let root = temp_root("hostile");
    let hostile = "https://user:secret@evil.example/?password=hunter2";
    let password = "s3c-password-must-not-leak";
    let error = test_runtime()
        .block_on(shared.login_with_password(
            "not-a-user".to_owned(),
            hostile.to_owned(),
            root.to_string_lossy().into_owned(),
            password.to_owned(),
        ))
        .expect_err("invalid identity");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s3c-identity-invalid"));
    assert!(!text.contains(password));
    assert!(!text.contains("hunter2"));
    assert!(!text.contains("secret"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn login_rejects_empty_password_without_echo() {
    let store = Box::new(MemoryCallbackVault(Arc::new(Mutex::new(HashMap::new()))));
    let shared = SharedCore::new_with_secret_store(store);
    let root = temp_root("empty-password");
    let error = test_runtime()
        .block_on(shared.login_with_password(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            root.to_string_lossy().into_owned(),
            String::new(),
        ))
        .expect_err("empty password");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s3c-login-failed"));
    assert!(!text.contains("password"));
    assert!(!text.contains("@alice"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn login_persist_path_writes_vault_keys_restorable_by_new_shared_core() {
    let access = "syt_s3c_login_persist_access";
    let refresh = "syr_s3c_login_persist_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    assert!(
        map.lock().expect("vault").is_empty(),
        "vault must start empty so persist writes the keys"
    );
    let writer = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let writer_root = temp_root("login-persist-writer");
    let reader_root = temp_root("login-persist-reader");
    let rt = test_runtime();
    let _enter = rt.enter();
    let dto = rt
        .block_on(writer.persist_planted_session_for_test(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            writer_root.to_string_lossy().into_owned(),
            "DEVICEABC".to_owned(),
            access.to_owned(),
            Some(refresh.to_owned()),
        ))
        .expect("production persist path wrote session material");
    assert_eq!(dto.user_id, "@alice:example.org");
    assert_eq!(dto.device_id, "DEVICEABC");
    assert_eq!(dto.homeserver_url, "https://matrix.example.org");
    let dto_text = format!("{dto:?}");
    assert!(!dto_text.contains(access));
    assert!(!dto_text.contains(refresh));
    let keys: Vec<String> = map.lock().expect("vault").keys().cloned().collect();
    assert!(keys.iter().any(|key| key.starts_with("matrix-session:")));
    assert!(keys.iter().any(|key| key.starts_with("store-key:")));
    assert!(!keys.iter().any(|key| key.contains("p4-s3b-store-key")));
    let retained_login = rt
        .block_on(writer.login_with_password(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            writer_root.to_string_lossy().into_owned(),
            "must-not-replace-retained-client".to_owned(),
        ))
        .expect_err("already retained after persist");
    let retained_text = format!("{retained_login:?}");
    assert!(retained_text.contains("p4-s3c-login-failed"));
    assert!(!retained_text.contains("must-not-replace-retained-client"));
    drop(writer);
    let reader = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let restored = rt
        .block_on(reader.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            reader_root.to_string_lossy().into_owned(),
        ))
        .expect("S3b restore after production login persist");
    assert_eq!(restored.user_id, "@alice:example.org");
    assert_eq!(restored.device_id, "DEVICEABC");
    let restored_text = format!("{restored:?}");
    assert!(!restored_text.contains(access));
    assert!(!restored_text.contains(refresh));
    let after_restore_login = rt
        .block_on(reader.login_with_password(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            reader_root.to_string_lossy().into_owned(),
            "must-not-login-after-restore".to_owned(),
        ))
        .expect_err("already restored");
    let after_restore_text = format!("{after_restore_login:?}");
    assert!(after_restore_text.contains("p4-s3c-login-failed"));
    assert!(!after_restore_text.contains("must-not-login-after-restore"));
    drop(reader);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&writer_root);
    let _ = fs::remove_dir_all(&reader_root);
}
