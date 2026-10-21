//! Store-root validation and the Swift-vault-backed store-key vault used by
//! SharedCore restore and login.

use super::*;

pub(super) fn parse_store_root(store_root: &str) -> Result<&Path, ()> {
    let trimmed = store_root.trim();
    if trimmed.is_empty() {
        return Err(());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(());
    }
    Ok(path)
}

pub(super) fn validate_store_root(store_root: &str) -> Result<&Path, SessionRestoreError> {
    parse_store_root(store_root)
        .map_err(|_| restore_failed(STORE_ROOT_INVALID_CODE, STORE_ROOT_INVALID_DESCRIPTION))
}

/// The account's store key: the vault's key (or a known older revision of
/// it) when present, and a new one only for an account root that does not
/// exist yet.
///
/// Call before `prepare_session_store`, which creates the account layout.
/// A missing or corrupt key for an existing store fails closed with
/// `STORE_RESET_REQUIRED_CODE`: iOS Keychain items do not move to a restored
/// device while the app-group store does, and a fresh key could never open
/// that store. Swift then offers `reset_local_account_store`.
pub(super) fn store_key_for(
    store: &Arc<dyn SecretVault + Send + Sync>,
    root: &Path,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let creation_policy = StorePaths::derive(root, identity)
        .and_then(|paths| paths.key_creation_policy())
        .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
    let vault = SecretStoreKeyVault {
        store: Arc::clone(store),
    };
    get_or_migrate_store_key(&vault, identity, creation_policy).map_err(|error| match error {
        StoreKeyVaultError::BackendUnavailable { .. } => {
            restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION)
        }
        StoreKeyVaultError::MissingKeyForExistingStore | StoreKeyVaultError::CorruptPayload => {
            store_reset_required()
        }
        _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
    })
}

pub(super) fn store_reset_required() -> SessionRestoreError {
    restore_failed(STORE_RESET_REQUIRED_CODE, STORE_RESET_REQUIRED_DESCRIPTION)
}

/// A client build that failed on the store itself (a key that does not open
/// it) needs the same reset as a missing key.
pub(super) fn store_build_needs_reset(
    error: &crate::app::client_builder::ClientBuilderError,
) -> bool {
    error.to_factory_error().category == MatrixIpcErrorCategory::StoreCorrupt
}

pub(super) struct SecretStoreKeyVault {
    pub(super) store: Arc<dyn SecretVault + Send + Sync>,
}

impl StoreKeyVault for SecretStoreKeyVault {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        match self.store.get(id.account()) {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) if bytes.len() == STORE_KEY_LEN => {
                let mut key_bytes = [0u8; STORE_KEY_LEN];
                key_bytes.copy_from_slice(&bytes);
                Ok(Some(StoreKeyMaterial::from_bytes(key_bytes)))
            }
            Ok(Some(_)) => Err(StoreKeyVaultError::CorruptPayload),
            Err(_) => Err(StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            }),
        }
    }

    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        self.store
            .put(id.account(), key.as_bytes().as_slice())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })
    }

    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store
            .delete(id.account())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })?;
        Ok(existed)
    }
}
