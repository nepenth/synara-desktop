//! Abstract vault for Matrix store encryption keys.
//!
//! The trait and in-memory harness live here so desktop and iOS can share
//! get-or-create policy. Live OS credential I/O (Keychain / Secret Service)
//! stays in the shell.

use std::collections::HashMap;
use std::sync::Mutex;

use super::identity::AccountIdentity;
use super::key_material::{StoreKeyId, StoreKeyMaterial, STORE_KEY_LEN, STORE_KEY_REVISION};
use super::paths::StoreKeyCreationPolicy;

/// Privacy-safe vault errors (never include key bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreKeyVaultError {
    /// No key stored for this id (caller may generate — must not wipe stores).
    NotFound,
    /// An existing account layout has no current or known legacy key.
    ///
    /// Key creation is forbidden here so an encrypted store can never be
    /// silently opened with replacement key material.
    MissingKeyForExistingStore,
    /// Backend unavailable / locked / denied.
    BackendUnavailable { diagnostic_id: &'static str },
    /// Stored payload corrupt or wrong length.
    CorruptPayload,
    /// Serialization / encoding failure.
    Encoding,
}

impl std::fmt::Display for StoreKeyVaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "store key not found"),
            Self::MissingKeyForExistingStore => {
                write!(f, "store key missing for existing store")
            }
            Self::BackendUnavailable { diagnostic_id } => {
                write!(f, "store key vault unavailable ({diagnostic_id})")
            }
            Self::CorruptPayload => write!(f, "store key payload corrupt"),
            Self::Encoding => write!(f, "store key encoding error"),
        }
    }
}

impl std::error::Error for StoreKeyVaultError {}

/// Read/write/delete store encryption keys by [`StoreKeyId`].
pub trait StoreKeyVault: Send {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError>;
    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError>;
    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError>;
}

/// Process-local vault for **unit/integration harnesses only**.
///
/// Must not be used as the production product vault (shells provide the OS
/// credential-store implementation).
#[derive(Debug, Default)]
pub struct InMemoryStoreKeyVault {
    inner: Mutex<HashMap<(String, String), [u8; STORE_KEY_LEN]>>,
}

impl InMemoryStoreKeyVault {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl StoreKeyVault for InMemoryStoreKeyVault {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p2.2-memory-vault-poisoned",
            })?;
        Ok(guard
            .get(&(id.service().to_owned(), id.account().to_owned()))
            .copied()
            .map(StoreKeyMaterial::from_bytes))
    }

    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p2.2-memory-vault-poisoned",
            })?;
        guard.insert(
            (id.service().to_owned(), id.account().to_owned()),
            *key.as_bytes(),
        );
        Ok(())
    }

    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p2.2-memory-vault-poisoned",
            })?;
        Ok(guard
            .remove(&(id.service().to_owned(), id.account().to_owned()))
            .is_some())
    }
}

/// Get-or-create helper: never deletes store dirs on vault miss.
pub fn get_or_create_store_key<V: StoreKeyVault + ?Sized>(
    vault: &V,
    id: &StoreKeyId,
) -> Result<StoreKeyMaterial, StoreKeyVaultError> {
    if let Some(existing) = vault.get(id)? {
        return Ok(existing);
    }
    let key = StoreKeyMaterial::generate().map_err(|_| StoreKeyVaultError::BackendUnavailable {
        diagnostic_id: "p2.2-entropy-unavailable",
    })?;
    vault.set(id, &key)?;
    Ok(key)
}

/// Read the current revision key or migrate one known older Keychain/Secret
/// Service entry forward without changing its bytes.
///
/// A new key is generated only for a genuinely fresh account root. Existing
/// account data with neither a current nor known legacy key fails closed.
pub fn get_or_migrate_store_key<V: StoreKeyVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
    creation_policy: StoreKeyCreationPolicy,
) -> Result<StoreKeyMaterial, StoreKeyVaultError> {
    let current = StoreKeyId::from_identity(identity);
    if let Some(key) = vault.get(&current)? {
        return Ok(key);
    }

    for revision in (1..STORE_KEY_REVISION).rev() {
        let Some(legacy) = StoreKeyId::for_revision(identity, revision) else {
            continue;
        };
        if legacy == current {
            continue;
        }
        if let Some(key) = vault.get(&legacy)? {
            vault.set(&current, &key)?;
            return Ok(key);
        }
    }

    match creation_policy {
        StoreKeyCreationPolicy::AllowForFreshStore => get_or_create_store_key(vault, &current),
        StoreKeyCreationPolicy::ForbidForExistingStore => {
            Err(StoreKeyVaultError::MissingKeyForExistingStore)
        }
    }
}

/// The store key for an account whose local store the user just reset with
/// archive-and-rebuild ([`super::reset_store_for_recovery`]).
///
/// The rebuilt layout is empty but its account root still exists, so
/// [`super::StorePaths::key_creation_policy`] keeps forbidding a new key and
/// a missing key would ask for the same reset forever. A key that is present
/// (current or a known older revision) is kept, so the archive stays
/// readable. A missing key, or a current entry whose bytes are corrupt, could
/// never open the archive, so the empty store gets a new key and loses
/// nothing. Call it only right after that user-confirmed reset: for any other
/// existing store a missing key must keep failing closed.
pub fn store_key_after_confirmed_reset<V: StoreKeyVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, StoreKeyVaultError> {
    let current = StoreKeyId::from_identity(identity);
    let forbid = StoreKeyCreationPolicy::ForbidForExistingStore;
    match get_or_migrate_store_key(vault, identity, forbid) {
        Err(StoreKeyVaultError::CorruptPayload) => {
            vault.delete(&current)?;
        }
        Err(StoreKeyVaultError::MissingKeyForExistingStore) => {}
        kept => return kept,
    }
    match get_or_migrate_store_key(vault, identity, forbid) {
        Err(StoreKeyVaultError::MissingKeyForExistingStore) => {
            get_or_create_store_key(vault, &current)
        }
        other => other,
    }
}

#[cfg(test)]
mod confirmed_reset_tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::app::store::{migrate_store_to_current, reset_store_for_recovery, StorePaths};

    fn identity() -> AccountIdentity {
        AccountIdentity::new("@reset:example.org", "https://example.org").unwrap()
    }

    fn rebuilt_store(label: &str) -> (std::path::PathBuf, StorePaths) {
        let root = std::env::temp_dir().join(format!(
            "synara-confirmed-reset-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        let paths = StorePaths::derive(&root, &identity()).unwrap();
        migrate_store_to_current(&paths).unwrap();
        std::fs::write(paths.state_dir().join("matrix-sdk-state.sqlite3"), b"old").unwrap();
        reset_store_for_recovery(&paths).unwrap();
        (root, paths)
    }

    /// The desktop follow-up from the crypto batch: archive-and-rebuild
    /// leaves the account root, so the creation policy alone can never
    /// finish recovering from a lost key.
    #[test]
    fn a_rebuilt_store_without_its_key_gets_a_new_one_only_through_the_reset() {
        let (root, paths) = rebuilt_store("missing");
        let vault = InMemoryStoreKeyVault::new();
        let policy = paths.key_creation_policy().unwrap();
        assert_eq!(policy, StoreKeyCreationPolicy::ForbidForExistingStore);
        assert_eq!(
            get_or_migrate_store_key(&vault, &identity(), policy).unwrap_err(),
            StoreKeyVaultError::MissingKeyForExistingStore,
            "the ordinary sign-in path would ask for the same reset again"
        );

        let key = store_key_after_confirmed_reset(&vault, &identity()).unwrap();
        // The next sign-in finds it under the unchanged, protective policy.
        let reopened = get_or_migrate_store_key(&vault, &identity(), policy).unwrap();
        assert_eq!(reopened.as_bytes(), key.as_bytes());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_present_key_survives_the_reset() {
        let (root, _paths) = rebuilt_store("present");
        let vault = InMemoryStoreKeyVault::new();
        let existing = StoreKeyMaterial::from_bytes([9; STORE_KEY_LEN]);
        vault
            .set(&StoreKeyId::from_identity(&identity()), &existing)
            .unwrap();
        let kept = store_key_after_confirmed_reset(&vault, &identity()).unwrap();
        assert_eq!(
            kept.as_bytes(),
            existing.as_bytes(),
            "the archive stays readable"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// Reads the current entry as corrupt until it is deleted.
    #[derive(Default)]
    struct CorruptCurrent {
        inner: InMemoryStoreKeyVault,
        deleted: AtomicBool,
    }

    impl StoreKeyVault for CorruptCurrent {
        fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
            if !self.deleted.load(Ordering::Acquire)
                && *id == StoreKeyId::from_identity(&identity())
            {
                return Err(StoreKeyVaultError::CorruptPayload);
            }
            self.inner.get(id)
        }

        fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
            self.inner.set(id, key)
        }

        fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
            self.deleted.store(true, Ordering::Release);
            self.inner.delete(id)
        }
    }

    #[test]
    fn a_corrupt_key_is_replaced_and_a_locked_vault_gets_nothing() {
        let vault = CorruptCurrent::default();
        let replaced = store_key_after_confirmed_reset(&vault, &identity()).unwrap();
        assert!(vault.deleted.load(Ordering::Acquire));
        assert_eq!(
            vault
                .get(&StoreKeyId::from_identity(&identity()))
                .unwrap()
                .unwrap()
                .as_bytes(),
            replaced.as_bytes()
        );

        struct Locked;
        impl StoreKeyVault for Locked {
            fn get(&self, _: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
                Err(StoreKeyVaultError::BackendUnavailable {
                    diagnostic_id: "test-locked",
                })
            }
            fn set(&self, _: &StoreKeyId, _: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
                panic!("a locked vault must not be written")
            }
            fn delete(&self, _: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
                panic!("a locked vault must not be cleared")
            }
        }
        assert!(matches!(
            store_key_after_confirmed_reset(&Locked, &identity()),
            Err(StoreKeyVaultError::BackendUnavailable { .. })
        ));
    }
}
