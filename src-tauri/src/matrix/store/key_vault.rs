//! Abstract vault for Matrix store encryption keys.
//!
//! Production will use the OS keyring; unit tests use [`InMemoryStoreKeyVault`].
//! Missing keys and IO failures must **not** delete on-disk Matrix stores
//! (plan §8.3 — no automatic wipe).

use std::collections::HashMap;
use std::sync::Mutex;

use super::key_material::{StoreKeyId, StoreKeyMaterial, STORE_KEY_LEN};

/// Privacy-safe vault errors (never include key bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreKeyVaultError {
    /// No key stored for this id (caller may generate — must not wipe stores).
    NotFound,
    /// Backend unavailable / locked / denied.
    BackendUnavailable {
        diagnostic_id: &'static str,
    },
    /// Stored payload corrupt or wrong length.
    CorruptPayload,
    /// Serialization / encoding failure.
    Encoding,
}

impl std::fmt::Display for StoreKeyVaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "store key not found"),
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

/// Process-local vault for unit/integration harnesses.
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

/// Non-secret service/account refs for a future keyring-backed vault.
///
/// Construction of a live `keyring::Entry` is deferred to production wiring
/// (still harness-only until Phase 3 session code). This type documents the
/// stable naming contract for P2.2 collision tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyringStoreKeyRefs {
    pub service: String,
    pub account: String,
}

impl KeyringStoreKeyRefs {
    pub fn from_id(id: &StoreKeyId) -> Self {
        Self {
            service: id.service().to_owned(),
            account: id.account().to_owned(),
        }
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
