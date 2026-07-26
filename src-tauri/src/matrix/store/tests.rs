//! Unit tests for P2.2 store paths and encryption-key foundation.

use super::*;
use std::fs;
use std::path::PathBuf;

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
fn layout_serializes_without_secrets() {
    let root = PathBuf::from("/var/app");
    let paths = StorePaths::derive(&root, &alice()).unwrap();
    let layout = paths.layout();
    let json = serde_json::to_string(&layout).unwrap();
    // camelCase wire
    assert!(json.contains("accountSegment"));
    assert!(json.contains("stateDir"));
    assert!(!json.contains("access_token"));
    assert!(!json.contains("storeKey"));
    let back: StoreLayout = serde_json::from_str(&json).unwrap();
    assert_eq!(back.account_segment, paths.account_segment());
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
fn account_segment_has_no_path_separators() {
    let id = AccountIdentity::new("@alice/evil:example.org", "https://example.org");
    // slash in localpart is rejected by validation
    assert!(id.is_err());

    let weird = AccountIdentity::new("@alice.bob-1:example.org", "https://example.org").unwrap();
    let seg = weird.account_dir_segment();
    assert!(!seg.contains('/'));
    assert!(!seg.contains('\\'));
    assert!(!seg.contains(".."));
}
