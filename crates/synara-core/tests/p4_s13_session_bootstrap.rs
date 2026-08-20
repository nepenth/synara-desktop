//! P4-S13: restore → attach → start on a fresh SharedCore that shares a vault.
//!
//! This is the cold-start product path. It is not iOS-on-engine and not
//! P4 acceptance. NSE still cannot start sync.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s13-it-{tag}-{nanos}"));
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
fn cold_start_restore_attach_start_on_fresh_core_is_privacy_safe() {
    let access = "syt_s13_bootstrap_access";
    let refresh = "syr_s13_bootstrap_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let root = temp_root("cold-start");
    let rt = test_runtime();
    let _enter = rt.enter();

    let planter =
        SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    rt.block_on(planter.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist");
    drop(planter);

    let fresh = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let restored = rt
        .block_on(fresh.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect("fresh core restores the planted vault session");
    let restored_text = format!("{restored:?}");
    assert!(!restored_text.contains(access));
    assert!(!restored_text.contains(refresh));
    assert!(!restored_text.contains("password"));

    rt.block_on(fresh.attach_session_owners())
        .expect("attach after restore");
    let started = rt
        .block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(15), fresh.start_sync())
                .await
                .expect("start_sync timed out")
        })
        .expect("start after restore+attach");
    let started_text = format!("{started:?}");
    assert!(started.session_generation > 0);
    assert_eq!(
        started.started,
        started.readiness == "running" || started.readiness == "offline",
        "Idle is not a live start: {}",
        started.readiness
    );
    assert!(!started_text.contains(access));
    assert!(!started_text.contains(refresh));
    assert!(!started_text.contains("@alice"));
    assert!(!started_text.contains("https://"));

    drop(fresh);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn retained_login_client_skips_restore_then_attach_start() {
    let access = "syt_s13_login_skip_restore_access";
    let refresh = "syr_s13_login_skip_restore_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let root = temp_root("login-skip-restore");
    let rt = test_runtime();
    let _enter = rt.enter();

    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist retains a Client");

    let restore = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("retained login client must skip restore");
    let restore_text = format!("{restore:?}");
    assert!(restore_text.contains("p4-s3b-session-already-restored"));
    assert!(!restore_text.contains("p4-s3b-restore-failed"));
    assert!(!restore_text.contains(access));
    assert!(!restore_text.contains(refresh));

    rt.block_on(shared.attach_session_owners())
        .expect("attach after retained login");
    let started = rt
        .block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
                .await
                .expect("start_sync timed out")
        })
        .expect("start after login-retain skip restore");
    let started_text = format!("{started:?}");
    assert!(started.session_generation > 0);
    assert_eq!(
        started.started,
        started.readiness == "running" || started.readiness == "offline",
        "Idle is not a live start: {}",
        started.readiness
    );
    assert!(!started_text.contains(access));
    assert!(!started_text.contains("@alice"));

    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bootstrap_without_vault_material_fails_closed_without_echo() {
    let identity = alice();
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::new(
        Mutex::new(HashMap::new()),
    ))));
    let root = temp_root("no-material");
    let rt = test_runtime();
    let restore = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("empty vault cannot restore");
    let attach = rt
        .block_on(shared.attach_session_owners())
        .expect_err("no session cannot attach");
    let start = rt
        .block_on(shared.start_sync())
        .expect_err("no attach cannot start");
    let text = format!("{restore:?}{attach:?}{start:?}");
    assert!(text.contains("p4-s3b-session-material-missing"));
    assert!(text.contains("p4-s3d-session-missing"));
    assert!(text.contains("p4-s12-sync-not-attached"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
    assert!(!text.contains("password"));
    drop(shared);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
