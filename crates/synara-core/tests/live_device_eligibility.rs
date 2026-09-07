//! Opt-in proof of the production fresh/restored session → device snapshot route.
//! Creates/revokes one dedicated test-account device; never changes device trust.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use synara_core::{DeviceSnapshotDto, IosSecretVault, IosSecretVaultError, SharedCore};

#[derive(Clone, Default)]
struct Vault(Arc<Mutex<HashMap<String, Vec<u8>>>>);
impl IosSecretVault for Vault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
        Ok(self.0.lock().unwrap().get(&key).cloned())
    }
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
        self.0.lock().unwrap().insert(key, value);
        Ok(())
    }
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
        self.0.lock().unwrap().remove(&key);
        Ok(())
    }
}

async fn observe(core: &SharedCore, phase: &str) -> Result<Option<bool>, &'static str> {
    core.attach_session_owners()
        .await
        .map_err(|_| "attach failed")?;
    core.start_sync().await.map_err(|_| "start failed")?;
    let result = core.device_snapshot().await;
    match result {
        Ok(DeviceSnapshotDto {
            own_verification,
            has_devices_to_verify_against,
            devices,
            ..
        }) => {
            // All output is closed vocabulary/counts: no account, device, URL, or keys.
            eprintln!("synara_eligibility_proof phase={phase} own={own_verification} eligibility={has_devices_to_verify_against:?} current_count={}", devices.iter().filter(|d| d.is_current).count());
            if devices.iter().filter(|d| d.is_current).count() != 1 {
                return Err("snapshot must include exactly one current device");
            }
            if own_verification == "verified" {
                return Err("fresh untrusted proof device unexpectedly verified");
            }
            if has_devices_to_verify_against.is_none() {
                return Err("eligibility unavailable");
            }
            Ok(has_devices_to_verify_against)
        }
        Err(error) => {
            // FFI error fields are source constants, not SDK diagnostics.
            eprintln!("synara_eligibility_proof phase={phase} error={error:?}");
            Err("snapshot failed")
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires authorized SYNARA_LIVE_* test credentials; creates/revokes only a fresh test device"]
async fn fresh_and_restored_core_report_device_eligibility() {
    let homeserver = std::env::var("SYNARA_LIVE_HOMESERVER").expect("live homeserver required");
    let username = std::env::var("SYNARA_LIVE_USERNAME").expect("live username required");
    let password = std::env::var("SYNARA_LIVE_PASSWORD").expect("live password required");
    let user_id = if username.starts_with('@') {
        username
    } else {
        let url = url::Url::parse(&homeserver).expect("valid live homeserver URL");
        format!("@{}:{}", username, url.host_str().expect("homeserver host"))
    };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-eligibility-proof-{nonce}"));
    std::fs::create_dir(&root).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let store = root.to_string_lossy().into_owned();
    let vault = Vault::default();
    let fresh = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let login = fresh
        .login_with_password(user_id.clone(), homeserver.clone(), store.clone(), password)
        .await
        .expect("production Core password login");
    let fresh_result = observe(&fresh, "fresh").await;
    fresh
        .logout()
        .await
        .expect("close fresh Core without remote logout");
    drop(fresh);
    let restored = SharedCore::new_with_secret_store(Box::new(vault));
    let restore_result = restored
        .restore_persisted_session(user_id.clone(), homeserver.clone(), store)
        .await;
    let restored_result = match restore_result {
        Ok(_) => observe(&restored, "restored").await,
        Err(_) => Err("restore failed"),
    };
    let revoked = restored
        .revoke_server_session(user_id, login.device_id, homeserver)
        .await;
    restored.logout().await.expect("close restored Core");
    drop(restored);
    std::fs::remove_dir_all(root).expect("remove disposable encrypted proof store");
    assert!(
        matches!(revoked, Ok(true)),
        "fresh proof session revocation failed"
    );
    let first = fresh_result.expect("fresh path");
    let second = restored_result.expect("restored path");
    assert_eq!(first, second, "eligibility changed across restore");
}
