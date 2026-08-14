//! P4-S9: typed SharedCore consume of verification SAS commands only.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-it-{tag}-{nanos}"));
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
fn verification_sas_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("verification_list"));
    assert!(udl.contains("verification_start"));
    assert!(udl.contains("verification_accept"));
    assert!(udl.contains("verification_begin_sas"));
    assert!(udl.contains("verification_confirm"));
    assert!(udl.contains("verification_mismatch"));
    assert!(udl.contains("verification_cancel"));
    assert!(udl.contains("verification_dismiss"));
    assert!(!udl.contains("matrix_device_snapshot"));
    assert!(!udl.contains("matrix_crypto_status"));
    assert!(!udl.contains("matrix_login_password"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("verification_list"));
    assert!(shared_core.contains("verification_start"));
    assert!(shared_core.contains("verification_accept"));
    assert!(shared_core.contains("verification_begin_sas"));
    assert!(shared_core.contains("verification_confirm"));
    assert!(shared_core.contains("verification_mismatch"));
    assert!(shared_core.contains("verification_cancel"));
    assert!(shared_core.contains("verification_dismiss"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("device_snapshot"));
    assert!(!shared_core.contains("crypto_status"));
}

#[test]
fn verification_sas_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let device_id = "DEVICE_S9_BOB";
    let flow_id = "$FLOW_S9_BOB";
    let start = rt
        .block_on(shared.verification_start(Some(device_id.to_owned())))
        .expect_err("no attached verification owner");
    let accept = rt
        .block_on(shared.verification_accept(flow_id.to_owned()))
        .expect_err("no attached verification owner");
    let begin_sas = rt
        .block_on(shared.verification_begin_sas(flow_id.to_owned()))
        .expect_err("no attached verification owner");
    let confirm = rt
        .block_on(shared.verification_confirm(flow_id.to_owned()))
        .expect_err("no attached verification owner");
    let mismatch = rt
        .block_on(shared.verification_mismatch(flow_id.to_owned()))
        .expect_err("no attached verification owner");
    let cancel = rt
        .block_on(shared.verification_cancel(flow_id.to_owned()))
        .expect_err("no attached verification owner");
    let dismiss = rt
        .block_on(shared.verification_dismiss(flow_id.to_owned()))
        .expect_err("no attached verification owner");
    let text = format!(
        "{start:?}{start}{accept:?}{accept}{begin_sas:?}{begin_sas}{confirm:?}{confirm}{mismatch:?}{mismatch}{cancel:?}{cancel}{dismiss:?}{dismiss}"
    );
    assert!(text.contains("p2-verification-start-no-session"));
    assert!(text.contains("p2-verification-accept-no-session"));
    assert!(text.contains("p2-verification-begin-sas-no-session"));
    assert!(text.contains("p2-verification-confirm-no-session"));
    assert!(text.contains("p2-verification-mismatch-no-session"));
    assert!(text.contains("p2-verification-cancel-no-session"));
    assert!(text.contains("p2-verification-dismiss-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(device_id));
    assert!(!text.contains(flow_id));
}

#[test]
fn verification_sas_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_verification_access";
    let refresh = "syr_s9_verification_refresh";
    let identity = alice();
    let device_id = "DEVICEABC";
    let flow_id = "$FLOW_S9_BOB";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("verification-sas-no-start");
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

    let start = rt
        .block_on(shared.verification_start(Some(device_id.to_owned())))
        .expect_err("unstarted sync still uses the registered start handler");
    let accept = rt
        .block_on(shared.verification_accept(flow_id.to_owned()))
        .expect_err("unstarted sync still uses the registered accept handler");
    let begin_sas = rt
        .block_on(shared.verification_begin_sas(flow_id.to_owned()))
        .expect_err("unstarted sync still uses the registered begin_sas handler");
    let confirm = rt
        .block_on(shared.verification_confirm(flow_id.to_owned()))
        .expect_err("unstarted sync still uses the registered confirm handler");
    let mismatch = rt
        .block_on(shared.verification_mismatch(flow_id.to_owned()))
        .expect_err("unstarted sync still uses the registered mismatch handler");
    let cancel = rt
        .block_on(shared.verification_cancel(flow_id.to_owned()))
        .expect_err("unstarted sync still uses the registered cancel handler");
    let dismiss = rt
        .block_on(shared.verification_dismiss(flow_id.to_owned()))
        .expect_err("unstarted sync still uses the registered dismiss handler");
    let start_text = format!("{start:?}{start}");
    let flow_text = format!(
        "{accept:?}{accept}{begin_sas:?}{begin_sas}{confirm:?}{confirm}{mismatch:?}{mismatch}{cancel:?}{cancel}{dismiss:?}{dismiss}"
    );
    let text = format!("{start_text}{flow_text}");
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    assert!(
        start_text.contains("v-crypto.1-") || start_text.contains("p4-s9-sas-failed"),
        "start must return a static mapped handler code: {start_text}"
    );
    assert!(
        flow_text.contains("v-crypto.1-flow-not-found"),
        "flow commands must return the registered missing-flow diagnostic: {flow_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(device_id));
    assert!(!text.contains(flow_id));
}
