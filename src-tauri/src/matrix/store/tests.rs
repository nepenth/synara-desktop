//! Unit tests for P2.2 store paths and encryption-key foundation.

use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_root(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("synara-p2.2-{}-{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    dir
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://example.org").unwrap()
}

fn bob() -> AccountIdentity {
    AccountIdentity::new("@bob:example.org", "https://example.org").unwrap()
}

fn alice_other_hs() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://matrix.other.org").unwrap()
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_store_markers(), MATRIX_STORE_MARKER);
}

#[test]
fn identity_validation_rejects_bad_inputs() {
    assert!(AccountIdentity::new("", "https://example.org").is_err());
    assert!(AccountIdentity::new("@alice:example.org", "").is_err());
    assert!(AccountIdentity::new("alice:example.org", "https://example.org").is_err());
    assert!(AccountIdentity::new("@alice", "https://example.org").is_err());
    assert!(AccountIdentity::new("@alice:example.org", "example.org").is_err());
    assert!(AccountIdentity::new("@alice:example.org", "https://ex..ample.org").is_err());
}

#[test]
fn identity_normalizes_trailing_slash() {
    let a = AccountIdentity::new("@alice:example.org", "https://example.org/").unwrap();
    let b = AccountIdentity::new("@alice:example.org", "https://example.org").unwrap();
    assert_eq!(a.canonical_key(), b.canonical_key());
    assert_eq!(a.account_dir_segment(), b.account_dir_segment());
}

#[test]
fn two_accounts_cannot_share_store_path_or_key_id() {
    let root = temp_root("collision");
    let paths_a = StorePaths::derive(&root, &alice()).unwrap();
    let paths_b = StorePaths::derive(&root, &bob()).unwrap();
    assert_ne!(paths_a.account_root(), paths_b.account_root());
    assert_ne!(paths_a.account_segment(), paths_b.account_segment());
    assert_ne!(paths_a.state_dir(), paths_b.state_dir());
    assert_ne!(paths_a.crypto_dir(), paths_b.crypto_dir());

    let key_a = StoreKeyId::from_identity(&alice());
    let key_b = StoreKeyId::from_identity(&bob());
    assert_ne!(key_a.account(), key_b.account());
    // Same service namespace, different accounts.
    assert_eq!(key_a.service(), key_b.service());
    assert_eq!(key_a.service(), STORE_KEY_SERVICE);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn same_user_different_homeserver_isolated() {
    let root = temp_root("hs-isolation");
    let a = StorePaths::derive(&root, &alice()).unwrap();
    let b = StorePaths::derive(&root, &alice_other_hs()).unwrap();
    assert_ne!(a.account_segment(), b.account_segment());
    assert_ne!(a.account_root(), b.account_root());
    assert_ne!(
        StoreKeyId::from_identity(&alice()).account(),
        StoreKeyId::from_identity(&alice_other_hs()).account()
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn same_identity_derives_stable_paths_and_key_ids() {
    let root = PathBuf::from("/tmp/synara-app-data");
    let p1 = StorePaths::derive(&root, &alice()).unwrap();
    let p2 = StorePaths::derive(&root, &alice()).unwrap();
    assert_eq!(p1, p2);
    assert_eq!(
        StoreKeyId::from_identity(&alice()),
        StoreKeyId::from_identity(&alice())
    );
}

#[test]
fn ensure_dirs_creates_layout_without_wiping_existing() {
    let root = temp_root("ensure");
    let paths = StorePaths::derive(&root, &alice()).unwrap();
    paths.ensure_dirs().unwrap();

    // Marker file in state dir must survive a second ensure_dirs (no wipe).
    let marker = paths.state_dir().join("keep-me.txt");
    fs::write(&marker, b"persist").unwrap();
    paths.ensure_dirs().unwrap();
    assert_eq!(fs::read(&marker).unwrap(), b"persist");

    assert!(paths.crypto_dir().is_dir());
    assert!(paths.cache_dir().is_dir());
    assert!(paths.media_dir().is_dir());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(paths.account_root())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn layout_serializes_without_secrets_or_absolute_paths() {
    let root = PathBuf::from("/var/app");
    let paths = StorePaths::derive(&root, &alice()).unwrap();
    let layout = paths.layout();
    let json = serde_json::to_string(&layout).unwrap();
    // camelCase wire + privacy-safe relative children only (R0.6 / REV-003)
    assert!(json.contains("accountSegment"));
    assert!(json.contains("relativeStateDir"));
    assert!(json.contains("confinedUnderMatrixRoot"));
    assert!(!json.contains("access_token"));
    assert!(!json.contains("storeKey"));
    assert!(!json.contains("/var/app"));
    assert!(!json.contains(paths.account_root().to_string_lossy().as_ref()));
    assert!(!json.contains(paths.state_dir().to_string_lossy().as_ref()));
    let back: StoreLayout = serde_json::from_str(&json).unwrap();
    assert_eq!(back.account_segment, paths.account_segment());
    assert_eq!(back.relative_state_dir, "state");
    assert!(back.confined_under_matrix_root);
}

#[test]
fn store_key_generate_and_redacted_debug() {
    let k = StoreKeyMaterial::generate().unwrap();
    let dbg = format!("{k:?}");
    assert!(dbg.contains("REDACTED"));
    assert!(!dbg.contains("0x"));
    // raw bytes must not appear as a long decimal dump of all zeros only check length
    assert_eq!(k.as_bytes().len(), STORE_KEY_LEN);
}

#[test]
fn memory_vault_round_trip_and_isolation() {
    let vault = InMemoryStoreKeyVault::new();
    let id_a = StoreKeyId::from_identity(&alice());
    let id_b = StoreKeyId::from_identity(&bob());

    let key_a = StoreKeyMaterial::generate().unwrap();
    vault.set(&id_a, &key_a).unwrap();
    assert!(vault.get(&id_b).unwrap().is_none());

    let loaded = vault.get(&id_a).unwrap().expect("alice key");
    assert!(loaded.equals(&key_a));

    assert!(vault.delete(&id_a).unwrap());
    assert!(vault.get(&id_a).unwrap().is_none());
}

#[test]
fn get_or_create_does_not_require_existing_store_dirs() {
    // Vault miss generates a key; no store paths are touched/deleted.
    let vault = InMemoryStoreKeyVault::new();
    let id = StoreKeyId::from_identity(&alice());
    let k1 = get_or_create_store_key(&vault, &id).unwrap();
    let k2 = get_or_create_store_key(&vault, &id).unwrap();
    assert!(k1.equals(&k2));
}

#[test]
fn missing_vault_key_is_not_found_not_wipe_signal() {
    let vault = InMemoryStoreKeyVault::new();
    let id = StoreKeyId::from_identity(&alice());
    assert!(vault.get(&id).unwrap().is_none());
    // Explicit: NotFound would be returned by APIs that require a key;
    // get returns None. Callers must not delete stores on either outcome.
}

#[test]
fn keyring_refs_stable_and_scoped() {
    let id = StoreKeyId::from_identity(&alice());
    let refs = KeyringStoreKeyRefs::from_id(&id);
    assert_eq!(refs.service, STORE_KEY_SERVICE);
    assert!(refs.account.starts_with("store-key:"));
    // Must not collide with session credential account name.
    assert_ne!(refs.account, "matrix-session");
}

#[test]
fn keyring_vault_platform_support_matches_cfg() {
    let supported = KeyringStoreKeyVault::platform_supported();
    assert_eq!(
        supported,
        cfg!(any(target_os = "macos", target_os = "linux"))
    );
}

/// R0.4 residual: live OS keyring round-trip when the backend is available.
///
/// Skips (returns early via Ok paths that tolerate unavailable backends) only
/// when the host cannot access the credential store — CI macOS/Linux runners
/// with a working keyring exercise the real path.
#[test]
fn keyring_vault_round_trip_when_available() {
    let vault = KeyringStoreKeyVault::new();
    let id = StoreKeyId::from_identity(
        &AccountIdentity::new(
            "@r04-keyring-probe:example.org",
            "https://matrix.example.org",
        )
        .unwrap(),
    );

    // Cleanup any leftover probe entry from a prior interrupted run.
    let _ = vault.delete(&id);

    let key = match StoreKeyMaterial::generate() {
        Ok(k) => k,
        Err(_) => return,
    };

    match vault.set(&id, &key) {
        Ok(()) => {}
        Err(StoreKeyVaultError::BackendUnavailable { .. }) => {
            // Backend locked/missing on this host — residual still implemented.
            return;
        }
        Err(e) => panic!("unexpected keyring set error: {e}"),
    }

    let loaded = vault
        .get(&id)
        .expect("get after set")
        .expect("key present after set");
    assert!(loaded.equals(&key));

    // get_or_create must not rotate an existing key.
    let again = get_or_create_store_key(&vault, &id).expect("get_or_create");
    assert!(again.equals(&key));

    assert!(vault.delete(&id).expect("delete"));
    assert!(vault.get(&id).expect("get after delete").is_none());
}

/// Encrypted store reopen using a key that survived a keyring get_or_create cycle.
#[test]
fn keyring_backed_encrypted_store_reopen_when_available() {
    use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let vault = KeyringStoreKeyVault::new();
    let identity = AccountIdentity::new(
        "@r04-reopen-probe:example.org",
        "https://matrix.example.org",
    )
    .unwrap();
    let id = StoreKeyId::from_identity(&identity);
    let _ = vault.delete(&id);

    let key = match get_or_create_store_key(&vault, &id) {
        Ok(k) => k,
        Err(StoreKeyVaultError::BackendUnavailable { .. }) => return,
        Err(e) => panic!("keyring get_or_create failed: {e}"),
    };
    // Reload from keyring (simulates process restart).
    let key_reloaded = match vault.get(&id) {
        Ok(Some(k)) => k,
        Ok(None) => panic!("key missing after get_or_create"),
        Err(StoreKeyVaultError::BackendUnavailable { .. }) => return,
        Err(e) => panic!("keyring get failed: {e}"),
    };
    assert!(key_reloaded.equals(&key));

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-r04-reopen-{nanos}"));
    let _ = fs::remove_dir_all(&root);

    let cfg1 = ClientBuildConfig::product_default(&root, identity.clone(), Some(key)).unwrap();
    let cfg2 = ClientBuildConfig::product_default(&root, identity, Some(key_reloaded)).unwrap();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let _enter = rt.enter();
    let c1 = rt
        .block_on(build_unauthenticated_client(&cfg1))
        .expect("first encrypted open");
    drop(c1);
    let c2 = rt
        .block_on(build_unauthenticated_client(&cfg2))
        .expect("reopen with keyring-reloaded key");
    assert!(c2.session().is_none());
    drop(c2);
    drop(_enter);
    drop(rt);

    let _ = vault.delete(&id);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn account_segment_has_no_path_separators() {
    let id = AccountIdentity::new("@alice/evil:example.org", "https://example.org");
    // slash in localpart is rejected by validation
    assert!(id.is_err());

    let weird = AccountIdentity::new("@alice.bob-1:example.org", "https://example.org").unwrap();
    let seg = weird.account_dir_segment();
    assert!(seg.starts_with("v1_"));
    assert!(!seg.contains('/'));
    assert!(!seg.contains('\\'));
    assert!(!seg.contains(".."));
    // SHA-256 prefix length (32 hex) after final underscore.
    let fp = seg.rsplit('_').next().unwrap();
    assert_eq!(fp.len(), 32);
    assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn derive_rejects_relative_app_data_root() {
    let err = StorePaths::derive(Path::new("relative/app-data"), &alice()).unwrap_err();
    assert!(matches!(err, StorePathError::RelativeAppDataRoot));
}

#[cfg(unix)]
#[test]
fn ensure_dirs_refuses_symlink_at_account_root() {
    use std::os::unix::fs::symlink;

    let root = temp_root("symlink-refuse");
    let paths = StorePaths::derive(&root, &alice()).unwrap();

    // Create a decoy outside the app-data root and point account_root at it.
    let outside = root.join("outside-target");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(paths.account_root().parent().unwrap()).unwrap();
    symlink(&outside, paths.account_root()).unwrap();

    let err = paths.ensure_dirs().unwrap_err();
    assert!(
        matches!(err, StorePathError::SymlinkRefused),
        "expected SymlinkRefused, got {err:?}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn key_revision_current_id_is_stable_and_known() {
    let identity = alice();
    let current = StoreKeyId::from_identity(&identity);
    assert_eq!(STORE_KEY_REVISION, 1);
    assert_eq!(current.service(), STORE_KEY_SERVICE_V1);
    assert_eq!(
        StoreKeyId::for_revision(&identity, STORE_KEY_REVISION),
        Some(current.clone())
    );
    assert_eq!(
        StoreKeyId::for_revision(&identity, STORE_KEY_REVISION + 1),
        None
    );
}

#[test]
fn current_revision_key_remains_usable_when_existing_store_disallows_generation() {
    // STORE_KEY_REVISION is currently the first key-id revision, so there is
    // no distinct legacy id to migrate in this build. A future revision adds
    // its historical mapping to `StoreKeyId::for_revision`; the current-key
    // path must remain usable while new-key creation is forbidden.
    let vault = InMemoryStoreKeyVault::new();
    let identity = alice();
    let id = StoreKeyId::from_identity(&identity);
    let existing = StoreKeyMaterial::generate().unwrap();
    vault.set(&id, &existing).unwrap();

    let resolved = get_or_migrate_store_key(
        &vault,
        &identity,
        StoreKeyCreationPolicy::ForbidForExistingStore,
    )
    .unwrap();
    assert!(resolved.equals(&existing));
    assert_eq!(vault.len(), 1, "current key must not be rotated");
}

#[derive(Debug)]
struct GetFailingVault {
    get_error: StoreKeyVaultError,
    set_attempts: std::sync::atomic::AtomicUsize,
}

impl GetFailingVault {
    fn new(get_error: StoreKeyVaultError) -> Self {
        Self {
            get_error,
            set_attempts: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn set_attempts(&self) -> usize {
        self.set_attempts.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl StoreKeyVault for GetFailingVault {
    fn get(&self, _id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        Err(self.get_error.clone())
    }

    fn set(&self, _id: &StoreKeyId, _key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        self.set_attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn delete(&self, _id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        Ok(false)
    }
}

#[test]
fn existing_encrypted_store_with_missing_key_fails_closed_without_replacement() {
    let root = temp_root("existing-key-missing");
    let identity = alice();
    let paths = StorePaths::derive(&root, &identity).unwrap();
    paths.ensure_dirs().unwrap();
    let encrypted_marker = paths.state_dir().join("matrix-sdk-state.sqlite3");
    fs::write(&encrypted_marker, b"encrypted sqlite bytes").unwrap();

    let policy = paths.key_creation_policy().unwrap();
    assert_eq!(policy, StoreKeyCreationPolicy::ForbidForExistingStore);
    let vault = InMemoryStoreKeyVault::new();
    let error = get_or_migrate_store_key(&vault, &identity, policy).unwrap_err();

    assert_eq!(error, StoreKeyVaultError::MissingKeyForExistingStore);
    assert!(vault.is_empty(), "a missing key must not be replaced");
    assert_eq!(
        fs::read(&encrypted_marker).unwrap(),
        b"encrypted sqlite bytes"
    );
    assert!(!error.to_string().contains(root.to_string_lossy().as_ref()));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn unavailable_keychain_for_existing_store_never_attempts_new_key_creation() {
    let root = temp_root("existing-keychain-locked");
    let identity = alice();
    let paths = StorePaths::derive(&root, &identity).unwrap();
    paths.ensure_dirs().unwrap();
    let encrypted_marker = paths.state_dir().join("matrix-sdk-crypto.sqlite3");
    fs::write(&encrypted_marker, b"encrypted crypto bytes").unwrap();

    let vault = GetFailingVault::new(StoreKeyVaultError::BackendUnavailable {
        diagnostic_id: "r0.4-keyring-no-storage-access",
    });
    let error = get_or_migrate_store_key(&vault, &identity, paths.key_creation_policy().unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        StoreKeyVaultError::BackendUnavailable { .. }
    ));
    assert_eq!(
        vault.set_attempts(),
        0,
        "locked Keychain must not be written"
    );
    assert_eq!(
        fs::read(&encrypted_marker).unwrap(),
        b"encrypted crypto bytes"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn corrupt_keychain_payload_for_existing_store_never_attempts_replacement() {
    let root = temp_root("existing-keychain-corrupt");
    let identity = alice();
    let paths = StorePaths::derive(&root, &identity).unwrap();
    paths.ensure_dirs().unwrap();
    let encrypted_marker = paths.state_dir().join("matrix-sdk-state.sqlite3");
    fs::write(&encrypted_marker, b"encrypted sqlite bytes").unwrap();

    let vault = GetFailingVault::new(StoreKeyVaultError::CorruptPayload);
    let error = get_or_migrate_store_key(&vault, &identity, paths.key_creation_policy().unwrap())
        .unwrap_err();

    assert_eq!(error, StoreKeyVaultError::CorruptPayload);
    assert_eq!(
        vault.set_attempts(),
        0,
        "corrupt Keychain data must not be replaced"
    );
    assert_eq!(
        fs::read(&encrypted_marker).unwrap(),
        b"encrypted sqlite bytes"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn fresh_store_first_run_creates_key_then_initializes_layout() {
    let root = temp_root("fresh-key-create");
    let identity = alice();
    let paths = StorePaths::derive(&root, &identity).unwrap();
    assert!(!paths.account_root().exists());
    assert_eq!(
        paths.key_creation_policy().unwrap(),
        StoreKeyCreationPolicy::AllowForFreshStore
    );
    assert!(
        !paths.account_root().exists(),
        "the preflight must not initialize a store layout"
    );

    let vault = InMemoryStoreKeyVault::new();
    let key = get_or_migrate_store_key(
        &vault,
        &identity,
        StoreKeyCreationPolicy::AllowForFreshStore,
    )
    .unwrap();
    assert_eq!(vault.len(), 1);
    migrate_store_to_current(&paths).unwrap();
    assert!(paths.account_root().is_dir());

    let reloaded = vault
        .get(&StoreKeyId::from_identity(&identity))
        .unwrap()
        .expect("fresh key stored");
    assert!(key.equals(&reloaded));
    let _ = fs::remove_dir_all(&root);
}
