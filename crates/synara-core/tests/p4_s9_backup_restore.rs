//! Typed SharedCore consume of live backup restore.
//!
//! Calls Core::restore_backup with the recovery secret as a method argument,
//! never Core::command JSON. Failed errors stay static and must not echo the
//! recovery key. Leftover `recover` stays on SharedCore.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-backup-restore-it-{tag}-{nanos}"));
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
fn backup_restore_surface_exposes_product_and_keeps_leftover_recover() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("restore_backup"));
    assert!(udl.contains("dictionary RestoreBackupDto"));
    assert!(udl.contains("interface RestoreBackupError"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("restore_backup("));
    assert!(shared_core.contains("recover("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_login_password"));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn backup_restore_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let recovery_key = "s9-it-recovery-key";

    let restore = rt
        .block_on(shared.restore_backup(recovery_key.to_owned()))
        .expect_err("no backup restore owner");

    let restore_text = format!("{restore:?}{restore}");
    assert!(restore_text.contains("p2-restore-backup-no-session"));
    assert!(!restore_text.contains("p4-s10-leftover-unavailable"));
    assert!(!restore_text.contains(recovery_key));
}

#[test]
fn backup_restore_empty_secret_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let error = test_runtime()
        .block_on(shared.restore_backup(String::new()))
        .expect_err("empty recovery secret must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("v-crypto.3-recovery-secret-empty"));
    assert!(!text.contains("p4-s10-leftover-unavailable"));
}

#[test]
fn backup_restore_oversize_key_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let marker = "s9ItOversizeRecoveryKey";
    let recovery_key = format!(
        "{marker}{}",
        "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let error = test_runtime()
        .block_on(shared.restore_backup(recovery_key.clone()))
        .expect_err("oversize recovery key must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s9-backup-restore-failed"));
    assert!(!text.contains(&recovery_key));
    assert!(!text.contains(marker));
}

#[test]
fn backup_restore_planted_session_returns_v_crypto_diagnostic_without_echo() {
    let access = "syt_s9_backup_restore_access";
    let refresh = "syr_s9_backup_restore_refresh";
    let recovery_key = "s9-it-planted-recovery-key";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("backup-restore-no-start");
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

    let restore = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            shared.restore_backup(recovery_key.to_owned()),
        )
        .await
        .expect("restore timed out")
    });
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let restore_text = match &restore {
        Ok(value) => format!("ok:{}", value.status),
        Err(error) => format!("{error:?}{error}"),
    };

    assert!(
        restore.is_ok() || restore_text.contains("v-crypto.3-"),
        "restore must return a handler or SDK diagnostic: {restore_text}"
    );
    assert!(
        !restore_text.contains("p4-s10-leftover-unavailable"),
        "restore must not use leftover-unavailable: {restore_text}"
    );
    assert!(
        !restore_text.contains("p4-s9-backup-restore-failed"),
        "restore must not hide a wrong envelope: {restore_text}"
    );
    assert!(!restore_text.contains(recovery_key));
    assert!(!restore_text.contains(access));
    assert!(!restore_text.contains(refresh));
    assert!(!restore_text.contains("syt_"));
}
