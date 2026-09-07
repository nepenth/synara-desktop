//! Opt-in proof of the production fresh/restored session → device snapshot route.
//! Creates/revokes one dedicated test-account device; never changes device trust.

use std::{
    collections::HashMap,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use synara_core::{
    app::{
        lifecycle::{HostMatrixSessionSecrets, SessionMaterial, SessionMaterialId},
        store::AccountIdentity,
    },
    DeviceSnapshotDto, IosSecretVault, IosSecretVaultError, SharedCore,
};

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

/// The product DTO intentionally still contains its existing local fallback.
/// A clean live proof must also assert the diagnostics from the *same* snapshot
/// call, not infer raw authority success from that DTO's Some(false).
fn authoritative_results(stderr: &str) -> Result<[bool; 2], &'static str> {
    let mut results = Vec::new();
    for line in stderr.lines() {
        let Some(record) = line.strip_prefix("synara_verification_snapshot ") else {
            continue;
        };
        match record {
            "authority=eligible sessions=available crypto=available" => results.push(true),
            "authority=none sessions=available crypto=available" => results.push(false),
            _ => return Err("snapshot used unavailable authority or enrichment"),
        }
    }
    results
        .try_into()
        .map_err(|_| "expected exactly two authoritative snapshots")
}

fn cleanup_material(
    vault: &Vault,
    identity: &AccountIdentity,
) -> Result<Option<HostMatrixSessionSecrets>, &'static str> {
    let key = SessionMaterialId::from_identity(identity);
    let bytes = vault
        .get(key.account().to_owned())
        .map_err(|_| "cleanup vault failed")?;
    let Some(bytes) = bytes else { return Ok(None) };
    let secrets = SessionMaterial::from_sealed_blob(bytes)
        .decode_host_secrets()
        .map_err(|_| "cleanup session material invalid")?;
    if secrets.user_id != identity.user_id()
        || AccountIdentity::new(&secrets.user_id, &secrets.homeserver_url)
            .ok()
            .as_ref()
            != Some(identity)
    {
        return Err("cleanup identity mismatch");
    }
    Ok(Some(secrets))
}

/// The vault retains only this fixture's newly created session. Local Core
/// logout intentionally releases its client, so keep this independent cleanup
/// authority until remote revocation is confirmed, including failed restore.
/// This emergency HTTP route is cleanup only, never eligibility evidence.
async fn revoke_retained_material(secrets: &HostMatrixSessionSecrets) -> bool {
    let Ok(base) = url::Url::parse(&secrets.homeserver_url) else {
        return false;
    };
    let Ok(endpoint) = base.join("_matrix/client/v3/logout") else {
        return false;
    };
    let Ok(http) = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    else {
        return false;
    };
    matches!(
        http.post(endpoint)
            .bearer_auth(&secrets.access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body("{}")
            .send()
            .await,
        Ok(response) if response.status().is_success()
    )
}

struct CleanupResult {
    revoked_by_core: bool,
    remote_revoked: bool,
    local_closed: bool,
    store_removed: bool,
}

async fn cleanup(
    fresh: &SharedCore,
    restored: &SharedCore,
    vault: &Vault,
    identity: &AccountIdentity,
    created_device_id: Option<&str>,
    root: &Path,
) -> CleanupResult {
    // Load before teardown. Neither restore failure nor logout failure may
    // discard the only authenticated authority for the disposable session.
    let material = cleanup_material(vault, identity).ok().flatten();
    let device_id = created_device_id.or_else(|| material.as_ref().map(|s| s.device_id.as_str()));
    let mut revoked_by_core = false;
    if let Some(device_id) = device_id {
        for core in [restored, fresh] {
            if matches!(
                core.revoke_server_session(
                    identity.user_id().to_owned(),
                    device_id.to_owned(),
                    identity.homeserver_url().to_owned(),
                )
                .await,
                Ok(true)
            ) {
                revoked_by_core = true;
                break;
            }
        }
    }
    let mut remote_revoked = revoked_by_core;
    if !remote_revoked {
        // A Core logout request may have refreshed authentication before it
        // failed. Prefer the vault's latest rotation over the earlier copy.
        let material = cleanup_material(vault, identity)
            .ok()
            .flatten()
            .or(material);
        if let Some(secrets) = material
            .as_ref()
            .filter(|s| created_device_id.is_none_or(|expected| s.device_id == expected))
        {
            remote_revoked = revoke_retained_material(secrets).await;
        }
    }
    // Always attempt both local teardowns, even when remote cleanup failed.
    let restored_closed = restored.logout().await.is_ok();
    let fresh_closed = fresh.logout().await.is_ok();
    // Keep the store when remote cleanup is not confirmed. Never claim success
    // or erase its local evidence before reporting a leftover server session.
    let store_removed = remote_revoked && std::fs::remove_dir_all(root).is_ok();
    CleanupResult {
        revoked_by_core,
        remote_revoked,
        local_closed: restored_closed && fresh_closed,
        store_removed,
    }
}

async fn run_live_fixture() -> Result<(), &'static str> {
    let homeserver =
        std::env::var("SYNARA_LIVE_HOMESERVER").map_err(|_| "live homeserver required")?;
    let username = std::env::var("SYNARA_LIVE_USERNAME").map_err(|_| "live username required")?;
    let password = std::env::var("SYNARA_LIVE_PASSWORD").map_err(|_| "live password required")?;
    let user_id = if username.starts_with('@') {
        username
    } else {
        let url = url::Url::parse(&homeserver).map_err(|_| "valid live homeserver URL required")?;
        format!(
            "@{}:{}",
            username,
            url.host_str().ok_or("homeserver host required")?
        )
    };
    let identity =
        AccountIdentity::new(&user_id, &homeserver).map_err(|_| "live identity invalid")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock invalid")?
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-eligibility-proof-{nonce}"));
    std::fs::create_dir(&root).map_err(|_| "proof store creation failed")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "proof store permissions failed")?;
    }
    let store = root.to_string_lossy().into_owned();
    let vault = Vault::default();
    let fresh = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let restored = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let login = fresh
        .login_with_password(user_id.clone(), homeserver.clone(), store.clone(), password)
        .await;
    // Do not use expect/panic/early return after the login attempt. Every path
    // joins the cleanup below, and keeps both Core handles plus the vault alive.
    let results = match &login {
        Ok(_) => {
            let first = observe(&fresh, "fresh").await;
            let second = if fresh.logout().await.is_ok() {
                match restored
                    .restore_persisted_session(user_id, homeserver, store)
                    .await
                {
                    Ok(_) => observe(&restored, "restored").await,
                    Err(_) => Err("restore failed"),
                }
            } else {
                Err("fresh local logout failed")
            };
            (first, second)
        }
        Err(_) => (Err("login failed"), Err("restore not attempted")),
    };
    let outcome = cleanup(
        &fresh,
        &restored,
        &vault,
        &identity,
        login.as_ref().ok().map(|l| l.device_id.as_str()),
        &root,
    )
    .await;
    eprintln!("synara_eligibility_proof cleanup_remote={} cleanup_core={} cleanup_local={} cleanup_store={}",
        outcome.remote_revoked, outcome.revoked_by_core, outcome.local_closed, outcome.store_removed);
    if !outcome.remote_revoked {
        return Err("proof server session revocation failed");
    }
    if !outcome.local_closed || !outcome.store_removed {
        return Err("proof local cleanup failed");
    }
    let first = results.0?;
    let second = results.1?;
    if !outcome.revoked_by_core {
        return Err("Core revocation required emergency cleanup");
    }
    if first != second {
        return Err("eligibility changed across restore");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires authorized SYNARA_LIVE_* test credentials; creates/revokes only a fresh test device"]
async fn fresh_and_restored_core_report_device_eligibility() {
    const CHILD: &str = "SYNARA_ELIGIBILITY_PROOF_CHILD";
    if std::env::var(CHILD).as_deref() == Ok("1") {
        assert!(
            run_live_fixture().await.is_ok(),
            "live fixture or cleanup failed"
        );
        return;
    }
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "fresh_and_restored_core_report_device_eligibility",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("SYNARA_VERIFICATION_DIAGNOSTICS", "1")
        .output()
        .expect("run isolated live proof");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Only echo the source-owned, bounded records, never arbitrary SDK output.
    for line in stderr.lines().filter(|line| {
        line.starts_with("synara_eligibility_proof ")
            || line.starts_with("synara_verification_snapshot ")
    }) {
        eprintln!("{line}");
    }
    assert!(output.status.success(), "live fixture or cleanup failed");
    let results =
        authoritative_results(&stderr).expect("raw authority must succeed on both snapshots");
    assert_eq!(
        results[0], results[1],
        "raw authority changed across restore"
    );
}

#[test]
fn clean_proof_rejects_local_fallback_even_when_snapshot_has_some_false() {
    let fallback = "synara_verification_snapshot authority=timeout sessions=available crypto=available\nsynara_eligibility_proof phase=fresh own=unverified eligibility=Some(false) current_count=1\n";
    assert!(authoritative_results(&fallback.repeat(2)).is_err());
    for failure in ["crypto_store", "server_response", "transport", "other"] {
        assert!(authoritative_results(&fallback.replace("timeout", failure).repeat(2)).is_err());
    }
    let none = "synara_verification_snapshot authority=none sessions=available crypto=available\n";
    let eligible = none.replace("authority=none", "authority=eligible");
    assert_eq!(authoritative_results(&none.repeat(2)), Ok([false, false]));
    assert_eq!(authoritative_results(&eligible.repeat(2)), Ok([true, true]));
    assert!(authoritative_results(none).is_err());
}

#[tokio::test]
async fn failed_restore_still_revokes_retained_fixture_authority() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    let server = MatrixMockServer::new().await;
    server
        .mock_logout()
        .expect_access_token("cleanup-fixture-token")
        .ok()
        .expect(1)
        .mount()
        .await;
    let identity = AccountIdentity::new("@cleanup:example.org", &server.uri()).unwrap();
    let material =
        SessionMaterial::from_matrix_tokens(&identity, "CREATED", "cleanup-fixture-token", None)
            .unwrap();
    let vault = Vault::default();
    vault
        .put(
            SessionMaterialId::from_identity(&identity)
                .account()
                .to_owned(),
            material.as_bytes().to_vec(),
        )
        .unwrap();
    let fresh = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let restored = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let root = std::env::temp_dir().join(format!(
        "synara-eligibility-cleanup-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    // Model local logout having released the fresh client and restoration
    // failing before adopting another one. The authenticated vault survives.
    assert!(restored
        .restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            "relative-store".to_owned()
        )
        .await
        .is_err());
    let outcome = cleanup(&fresh, &restored, &vault, &identity, Some("CREATED"), &root).await;
    assert!(!outcome.revoked_by_core);
    assert!(outcome.remote_revoked);
    assert!(outcome.local_closed);
    assert!(outcome.store_removed);
    assert!(!root.exists());
}

#[tokio::test]
async fn failed_remote_cleanup_is_not_success_and_does_not_remove_store() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    let server = MatrixMockServer::new().await;
    server
        .mock_logout()
        .expect_access_token("cleanup-fixture-token")
        .error500()
        .expect(1)
        .mount()
        .await;
    let identity = AccountIdentity::new("@cleanup:example.org", &server.uri()).unwrap();
    let material =
        SessionMaterial::from_matrix_tokens(&identity, "CREATED", "cleanup-fixture-token", None)
            .unwrap();
    let vault = Vault::default();
    vault
        .put(
            SessionMaterialId::from_identity(&identity)
                .account()
                .to_owned(),
            material.as_bytes().to_vec(),
        )
        .unwrap();
    let fresh = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let restored = SharedCore::new_with_secret_store(Box::new(vault.clone()));
    let root = std::env::temp_dir().join(format!(
        "synara-eligibility-cleanup-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let outcome = cleanup(&fresh, &restored, &vault, &identity, Some("CREATED"), &root).await;
    assert!(!outcome.remote_revoked);
    assert!(outcome.local_closed);
    assert!(!outcome.store_removed);
    assert!(root.exists());
    // This test uses only synthetic credentials; remove its intentional failure evidence.
    std::fs::remove_dir_all(root).unwrap();
}
