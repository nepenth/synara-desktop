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
