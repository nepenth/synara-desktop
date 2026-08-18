//! P4-S3d: attach the desktop owner set on a retained SharedCore Client.
//!
//! This is not `Core.command` and does not register leftovers.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s3d-it-{tag}-{nanos}"));
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

fn expected_owners() -> Vec<String> {
    vec![
        "typing".to_owned(),
        "presence".to_owned(),
        "verification".to_owned(),
        "devices".to_owned(),
        "join_rules".to_owned(),
        "image_packs".to_owned(),
        "timelines".to_owned(),
        "sync".to_owned(),
    ]
}

#[test]
fn attach_surface_does_not_register_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("attach_session_owners"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("attach_typing"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(!shared_core.contains(" command("));
}

#[test]
fn attach_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let error = test_runtime()
        .block_on(shared.attach_session_owners())
        .expect_err("no retained client");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s3d-session-missing"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
}

#[test]
fn attach_after_planted_persist_then_second_attach_fails_closed() {
    let access = "syt_s3d_attach_persist_access";
    let refresh = "syr_s3d_attach_persist_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("attach-after-persist");
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
    let dto = rt
        .block_on(shared.attach_session_owners())
        .expect("attach after retained session");
    assert_eq!(dto.owners, expected_owners());
    let dto_text = format!("{dto:?}");
    assert!(!dto_text.contains(access));
    assert!(!dto_text.contains(refresh));
    assert!(!dto_text.contains("password"));
    let second = rt
        .block_on(shared.attach_session_owners())
        .expect_err("second attach");
    let second_text = format!("{second:?}{second}");
    assert!(second_text.contains("p4-s3d-already-attached"));
    assert!(!second_text.contains(access));
    assert!(!second_text.contains(refresh));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
