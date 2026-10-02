//! Unit + temp-dir integration tests for P2.6 destructive lifecycle.

use super::*;

use crate::matrix::store::{
    get_or_create_store_key, AccountIdentity, InMemoryStoreKeyVault, StoreKeyId, StorePaths,
};

use std::fs;

use std::path::{Path, PathBuf};

fn temp_root(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "synara-p2.6-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    fs::canonicalize(&dir).unwrap_or(dir)
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://example.org").unwrap()
}

fn bob() -> AccountIdentity {
    AccountIdentity::new("@bob:example.org", "https://example.org").unwrap()
}

fn seed_account_store(root: &Path, identity: &AccountIdentity) -> StorePaths {
    let paths = StorePaths::derive(root, identity).unwrap();
    paths.ensure_dirs().unwrap();
    fs::write(paths.state_dir().join("state.db"), b"state-blob").unwrap();
    fs::write(paths.crypto_dir().join("crypto.db"), b"crypto-blob").unwrap();
    fs::write(paths.cache_dir().join("cache.bin"), b"cache-blob").unwrap();
    fs::write(paths.media_dir().join("m1"), b"media-blob").unwrap();
    paths
}

#[test]
fn wipe_target_refuses_relative_app_data_root() {
    let err = WipeTarget::resolve(Path::new("relative/root"), alice()).unwrap_err();
    assert!(matches!(
        err,
        LifecycleError::InvalidTarget {
            diagnostic_id: "p2.6-relative-app-data-root"
        }
    ));
}

#[test]
fn wipe_target_refuses_empty_app_data_root() {
    let err = WipeTarget::resolve(Path::new(""), alice()).unwrap_err();
    assert!(matches!(
        err,
        LifecycleError::InvalidTarget {
            diagnostic_id: "p2.6-empty-app-data-root"
        }
    ));
}

#[test]
fn wrong_path_refused_by_exact_target_checks() {
    let root = temp_root("wrong-path");
    let target = WipeTarget::resolve(&root, alice()).unwrap();

    let sibling = root.join("matrix").join("not-alice-segment");
    assert!(assert_path_is_wipe_allowed(&target, &sibling).is_err());
    assert!(assert_path_is_wipe_allowed(&target, target.matrix_root()).is_err());
    assert!(assert_path_is_wipe_allowed(&target, target.app_data_root()).is_err());
    assert!(assert_path_is_wipe_allowed(&target, Path::new("/tmp")).is_err());

    // Exact account root is the only allowed wipe path.
    assert!(assert_exact_account_root(&target).is_ok());
    assert!(assert_path_is_wipe_allowed(&target, target.account_root()).is_ok());
    // Child subdirs alone are not the wipe entrypoint (whole account root is).
    assert!(assert_path_is_wipe_allowed(&target, target.paths().state_dir()).is_err());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn wipe_removes_exact_account_only_sibling_untouched() {
    let root = temp_root("sibling");
    let alice_paths = seed_account_store(&root, &alice());
    let bob_paths = seed_account_store(&root, &bob());

    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let vault = InMemoryStoreKeyVault::new();
    let report = wipe_account_store(&target, Some(&vault)).unwrap();

    assert_eq!(report.account_segment, alice_paths.account_segment());
    assert_eq!(report.wipe_target_kind, WIPE_TARGET_KIND_ACCOUNT_ROOT);
    // R0.6 / REV-003: wipe report must not embed absolute paths.
    let report_dbg = format!("{report:?}");
    assert!(!report_dbg.contains(root.to_string_lossy().as_ref()));
    assert!(!report_dbg.contains(alice_paths.account_root().to_string_lossy().as_ref()));
    assert!(!alice_paths.account_root().exists());
    assert!(!alice_paths.state_dir().exists());

    assert!(bob_paths.account_root().is_dir());
    assert_eq!(
        fs::read(bob_paths.state_dir().join("state.db")).unwrap(),
        b"state-blob"
    );
    assert_eq!(
        fs::read(bob_paths.crypto_dir().join("crypto.db")).unwrap(),
        b"crypto-blob"
    );
    assert!(root.join("matrix").is_dir());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn wipe_is_idempotent_when_already_absent() {
    let root = temp_root("idempotent");
    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let vault = InMemoryStoreKeyVault::new();
    let report = wipe_account_store(&target, Some(&vault)).unwrap();
    assert!(report.account_root_removed);

    let report2 = wipe_account_store(&target, Some(&vault)).unwrap();
    assert!(report2.account_root_removed);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn session_material_service_distinct_from_store_key() {
    let id = SessionMaterialId::from_identity(&alice());
    let store_id = StoreKeyId::from_identity(&alice());
    assert_eq!(id.service(), SESSION_MATERIAL_SERVICE);
    assert_ne!(id.service(), store_id.service());
    assert!(id.account().starts_with("matrix-session:"));
    assert!(store_id.account().starts_with("store-key:"));
}

#[test]
fn session_material_vault_clear_hooks() {
    let vault = InMemorySessionMaterialVault::new();
    let id = SessionMaterialId::from_identity(&alice());
    vault
        .set(&id, &SessionMaterial::from_placeholder(b"placeholder"))
        .unwrap();
    assert!(vault.get(&id).unwrap().is_some());
    assert!(clear_session_material(&vault, &alice()).unwrap());
    assert!(vault.get(&id).unwrap().is_none());
    assert!(!clear_session_material(&vault, &alice()).unwrap());
}

// --- R0.7 slice 3: composed encrypted-store + supervisor lifecycle (real SDK) ---

/// Identity for R0.7 composed residual. Loopback-shaped homeserver URL (no
/// live network required for unauthenticated Client open). Optional live
/// disposable-Synapse URL via `SYNARA_MATRIX_HOMESERVER_URL` when gated.
fn r0_7_identity() -> AccountIdentity {
    let hs = std::env::var("SYNARA_MATRIX_HOMESERVER_URL")
        .ok()
        .filter(|u| {
            std::env::var("SYNARA_RUN_MATRIX_RUST_AUTH_LIVE")
                .ok()
                .as_deref()
                == Some("1")
                && {
                    let Ok(parsed) = url::Url::parse(u) else {
                        return false;
                    };
                    parsed.scheme() == "http"
                        && matches!(
                            parsed.host_str(),
                            Some("127.0.0.1") | Some("localhost") | Some("::1")
                        )
                        && parsed.username().is_empty()
                        && parsed.password().is_none()
                }
        })
        .unwrap_or_else(|| "http://127.0.0.1:8008".to_owned());
    AccountIdentity::new("@r07-lifecycle:localhost", &hs).unwrap()
}

/// R0.7 slice 4 — always-on residual: wrong store-key reopen fails with a
/// privacy-safe builder error (no absolute paths, no key material, no raw SDK
/// dump). Complements happy-path reopen in slice 3.
#[test]
fn r0_7_wrong_store_key_reopen_fails_privately() {
    use crate::matrix::client_builder::{
        build_unauthenticated_client, ClientBuildConfig, ClientBuilderError,
    };
    use crate::matrix::store::StoreKeyMaterial;

    let root = temp_root("r07-wrong-key");
    let identity = r0_7_identity();
    let key_a = StoreKeyMaterial::generate().unwrap();
    let key_b = StoreKeyMaterial::generate().unwrap();
    assert!(!key_a.equals(&key_b));
    let key_a_hex = key_a
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let key_b_hex = key_b
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root_display = root.display().to_string();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("multi-thread runtime for SDK Client drop");
    let _enter = rt.enter();

    let cfg_ok = ClientBuildConfig::product_default(&root, identity.clone(), Some(key_a)).unwrap();
    let client = rt
        .block_on(build_unauthenticated_client(&cfg_ok))
        .expect("first encrypted open with key A");
    assert!(client.session().is_none());
    drop(client);

    let cfg_bad = ClientBuildConfig::product_default(&root, identity.clone(), Some(key_b)).unwrap();
    let err = rt
        .block_on(build_unauthenticated_client(&cfg_bad))
        .expect_err("wrong store key must fail reopen");

    match &err {
        ClientBuilderError::SdkBuild {
            diagnostic_id,
            message,
            category: _,
        } => {
            assert!(
                diagnostic_id.starts_with("p2.3-"),
                "expected redacted p2.3 diagnostic id, got {diagnostic_id}"
            );
            assert!(!message.is_empty(), "safe message must be non-empty");
            // Privacy: Display surface must not leak paths or key material.
            let surface = err.to_string();
            assert!(
                !surface.contains(&root_display),
                "error must not contain absolute store root"
            );
            assert!(
                !surface.contains(&key_a_hex),
                "error must not contain key A"
            );
            assert!(
                !surface.contains(&key_b_hex),
                "error must not contain key B"
            );
            assert!(
                !surface.contains("sqlite"),
                "error must not leak raw engine detail: {surface}"
            );
        }
        other => panic!("expected SdkBuild error, got {other:?}"),
    }

    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

/// Real encrypted stores survive client teardown and reopen, then explicit
/// verified wipe removes only the selected account and native store key.
#[test]
fn r0_7_encrypted_store_open_logout_reopen_wipe() {
    use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
    use crate::matrix::store::StoreKeyMaterial;
    let root = temp_root("r07-native-store-lifecycle");
    let identity = r0_7_identity();
    let key_bytes = *StoreKeyMaterial::generate().unwrap().as_bytes();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let _enter = rt.enter();
    let config = ClientBuildConfig::product_default(
        &root,
        identity.clone(),
        Some(StoreKeyMaterial::from_bytes(key_bytes)),
    )
    .unwrap();
    let client = rt.block_on(build_unauthenticated_client(&config)).unwrap();
    assert!(client.session().is_none());
    let sessions = InMemorySessionMaterialVault::new();
    let session_id = SessionMaterialId::from_identity(&identity);
    sessions
        .set(
            &session_id,
            &SessionMaterial::from_placeholder(b"r07-native-session"),
        )
        .unwrap();
    drop(client);
    assert!(clear_session_material(&sessions, &identity).unwrap());
    assert!(sessions.get(&session_id).unwrap().is_none());
    assert!(config.state_store_path().is_dir());
    let reopened = rt.block_on(build_unauthenticated_client(&config)).unwrap();
    assert!(reopened.session().is_none());
    drop(reopened);
    let keys = InMemoryStoreKeyVault::new();
    let key_id = StoreKeyId::from_identity(&identity);
    get_or_create_store_key(&keys, &key_id).unwrap();
    let sibling = seed_account_store(&root, &bob());
    let target = WipeTarget::resolve(&root, identity).unwrap();
    let report = wipe_account_store(&target, Some(&keys)).unwrap();
    assert!(report.account_root_removed);
    assert!(report.store_key_removed);
    assert!(!target.account_root().exists());
    assert!(sibling.account_root().is_dir());
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(root);
}

/// Exercise SharedCore queue clearing over a retained SDK session. This
/// regression covers teardown and reuse of the polling queue; it does not
/// attach owners or claim session-generation advancement/rejection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn r0_7_owner_queue_teardown_after_real_sdk_logout() {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore};
    #[derive(Default)]
    struct MemoryCallbackVault(Mutex<HashMap<String, Vec<u8>>>);
    impl IosSecretVault for MemoryCallbackVault {
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
    let root = temp_root("r07-native-stale-owner");
    let identity = r0_7_identity();
    let core = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault::default()));
    core.persist_planted_session_for_test(
        identity.user_id().into(),
        identity.homeserver_url().into(),
        root.to_string_lossy().into_owned(),
        "DEVICE".into(),
        "fixture-token".into(),
        None,
    )
    .await
    .unwrap();
    let generation = core
        .session_snapshot()
        .await
        .unwrap()
        .session_generation
        .unwrap();
    core.enqueue_owner_update_for_test("typing".into(), generation, None);
    assert_eq!(core.poll_owner_updates().await.unwrap().len(), 1);
    core.enqueue_owner_update_for_test("typing".into(), generation, None);
    core.logout().await.unwrap();
    assert!(core.poll_owner_updates().await.unwrap().is_empty());
    let logged_out = core.session_snapshot().await.unwrap();
    assert_eq!(logged_out.status, "logged_out");
    assert!(logged_out.session_generation.is_none());
    core.restore_persisted_session(
        identity.user_id().into(),
        identity.homeserver_url().into(),
        root.to_string_lossy().into_owned(),
    )
    .await
    .unwrap();
    let restored = core
        .session_snapshot()
        .await
        .unwrap()
        .session_generation
        .unwrap();
    core.enqueue_owner_update_for_test("typing".into(), restored, None);
    assert_eq!(core.poll_owner_updates().await.unwrap().len(), 1);
    core.logout().await.unwrap();
    drop(core);
    let _ = fs::remove_dir_all(root);
}
