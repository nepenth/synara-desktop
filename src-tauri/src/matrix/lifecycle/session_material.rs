//! Session material hooks (harness / mock keyring).
//!
//! Logout clears session credentials for an account. Local store wipe may also
//! clear them. Neither path starts a production login/sync loop or dual-backend.
//!
//! Distinct from store-encryption key vault (`StoreKeyVault` / `store-key:…`).

use std::collections::HashMap;
use std::sync::Mutex;

use crate::matrix::store::AccountIdentity;

use super::error::LifecycleError;

/// Credential service name for Matrix session material (native only).
///
/// Intentionally distinct from `STORE_KEY_SERVICE`.
pub const SESSION_MATERIAL_SERVICE: &str =
    "com.whylandcreative.synara.desktop.matrix-session";

/// Non-secret keyring account id for session material for one account.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionMaterialId {
    service: String,
    account: String,
}

impl SessionMaterialId {
    pub fn from_identity(identity: &AccountIdentity) -> Self {
        Self {
            service: SESSION_MATERIAL_SERVICE.to_owned(),
            account: format!("matrix-session:{}", identity.account_dir_segment()),
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn account(&self) -> &str {
        &self.account
    }
}

/// Opaque session material for harness tests (not production tokens on wire).
///
/// Real access/refresh tokens never leave the vault abstraction in product code;
/// tests may store placeholder blobs only.
#[derive(Clone)]
pub struct SessionMaterial {
    /// Opaque non-secret test label or sealed placeholder (never logged as token).
    blob: Vec<u8>,
}

impl SessionMaterial {
    pub fn from_placeholder(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            blob: bytes.into(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.blob
    }

    pub fn len(&self) -> usize {
        self.blob.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }
}

impl std::fmt::Debug for SessionMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionMaterial([REDACTED len={}])", self.blob.len())
    }
}

impl Drop for SessionMaterial {
    fn drop(&mut self) {
        for b in &mut self.blob {
            unsafe {
                std::ptr::write_volatile(b, 0);
            }
        }
        self.blob.clear();
    }
}

/// Read/write/clear session credentials by account identity.
pub trait SessionMaterialVault: Send {
    fn get(&self, id: &SessionMaterialId) -> Result<Option<SessionMaterial>, LifecycleError>;
    fn set(&self, id: &SessionMaterialId, material: &SessionMaterial) -> Result<(), LifecycleError>;
    /// Remove session material. Returns whether an entry existed.
    fn clear(&self, id: &SessionMaterialId) -> Result<bool, LifecycleError>;
}

/// Process-local mock keyring for unit/integration harnesses.
#[derive(Debug, Default)]
pub struct InMemorySessionMaterialVault {
    inner: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl InMemorySessionMaterialVault {
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

impl SessionMaterialVault for InMemorySessionMaterialVault {
    fn get(&self, id: &SessionMaterialId) -> Result<Option<SessionMaterial>, LifecycleError> {
        let guard = self.inner.lock().map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p2.6-session-vault-poisoned",
            category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
        })?;
        Ok(guard
            .get(&(id.service().to_owned(), id.account().to_owned()))
            .cloned()
            .map(SessionMaterial::from_placeholder))
    }

    fn set(&self, id: &SessionMaterialId, material: &SessionMaterial) -> Result<(), LifecycleError> {
        let mut guard = self.inner.lock().map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p2.6-session-vault-poisoned",
            category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
        })?;
        guard.insert(
            (id.service().to_owned(), id.account().to_owned()),
            material.as_bytes().to_vec(),
        );
        Ok(())
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, LifecycleError> {
        let mut guard = self.inner.lock().map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p2.6-session-vault-poisoned",
            category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
        })?;
        Ok(guard
            .remove(&(id.service().to_owned(), id.account().to_owned()))
            .is_some())
    }
}

/// Clear session material for `identity` via the vault hook.
pub fn clear_session_material<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
) -> Result<bool, LifecycleError> {
    let id = SessionMaterialId::from_identity(identity);
    vault.clear(&id)
}
