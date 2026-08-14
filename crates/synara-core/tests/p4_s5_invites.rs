//! P4-S5: typed SharedCore consume of `matrix_invites_snapshot` only.
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
    let root = std::env::temp_dir().join(format!("synara-p4-s5-it-{tag}-{nanos}"));
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
fn invites_surface_exposes_only_the_registered_snapshot_command() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("invites_snapshot"));
    assert!(!udl.contains("matrix_invites_decline"));
    assert!(!udl.contains("matrix_invites_report_spam"));
    assert!(!udl.contains("matrix_invites_block_sender"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("attach_typing"));
    assert!(!udl.contains("matrix_send_sticker"));
    assert!(!udl.contains("matrix_send_poll"));
    assert!(!udl.contains("matrix_edit_message"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("invites_snapshot"));
    assert!(shared_core.contains("room_list_snapshot"));
    assert!(!shared_core.contains("command("));
}

#[test]
fn invites_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let error = test_runtime()
        .block_on(shared.invites_snapshot())
        .expect_err("no attached join-rule owner");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p2-invites-snapshot-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
}

#[test]
fn invites_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s5_invites_access";
    let refresh = "syr_s5_invites_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("invites-no-start");
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
        .block_on(shared.invites_snapshot())
        .expect("unstarted sync yields the registered handler result");
    assert_eq!(dto.session_generation, 1);
    assert!(dto.invites.is_empty());
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
