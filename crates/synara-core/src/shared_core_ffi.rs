//! UniFFI construction and restore facade for the shared Core.
//!
//! P4-S2 exposed only `SharedCore::new` with a fail-closed vault. P4-S3a adds
//! `new_with_secret_store` so Swift can install a Keychain-backed
//! [`SecretVault`]. P4-S3b adds `restore_persisted_session`: load sealed
//! session material from that vault and restore it onto a Core-built Client.
//! This still exposes no command, attach, password, or APNs surface.
//! This is not the desktop leftover `matrix_restore_session`.

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use matrix_sdk::Client;

use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::app::lifecycle::{
    restore_session_from_vault, SessionMaterial, SessionMaterialId, SessionMaterialVault,
};
use crate::app::store::{AccountIdentity, StoreKeyMaterial, STORE_KEY_LEN};
use crate::core::Core;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{IosFailClosedPlatform, Platform, SecretVault};
use crate::transport::{MatrixIpcError, MatrixIpcErrorCategory};

const VAULT_UNAVAILABLE_CODE: &str = "p4-s3b-secret-vault-unavailable";
const VAULT_UNAVAILABLE_DESCRIPTION: &str = "The secret store is unavailable.";
const IDENTITY_INVALID_CODE: &str = "p4-s3b-identity-invalid";
const IDENTITY_INVALID_DESCRIPTION: &str = "The session identity is invalid.";
const STORE_ROOT_INVALID_CODE: &str = "p4-s3b-store-root-invalid";
const STORE_ROOT_INVALID_DESCRIPTION: &str = "The session store root is invalid.";
const MATERIAL_MISSING_CODE: &str = "p4-s3b-session-material-missing";
const MATERIAL_MISSING_DESCRIPTION: &str = "No restorable session is available.";
const RESTORE_FAILED_CODE: &str = "p4-s3b-restore-failed";
const RESTORE_FAILED_DESCRIPTION: &str = "The persisted session could not be restored.";
const STORE_KEY_PREFIX: &str = "p4-s3b-store-key:";

/// Static fail-closed vault error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IosSecretVaultError {
    Unavailable { code: String, description: String },
}

impl std::fmt::Display for IosSecretVaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IosSecretVaultError {}

/// Swift-owned key/value callback. UniFFI UDL scaffolding consumes its
/// generated trait stub, so the crate must define this surface itself.
pub trait IosSecretVault: Send + Sync {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError>;
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError>;
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError>;
}

/// Privacy-safe restore outcome. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestoreDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed restore error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRestoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionRestoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionRestoreError {}

fn restore_failed(code: &'static str, description: &'static str) -> SessionRestoreError {
    SessionRestoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Retained shared Core for the iOS UniFFI boundary.
pub struct SharedCore {
    core: Core,
    secret_store: Arc<dyn SecretVault + Send + Sync>,
    restored_client: Mutex<Option<Client>>,
}

impl SharedCore {
    /// Construct a real Core with the fail-closed iOS Platform.
    pub fn new() -> Self {
        let platform = IosFailClosedPlatform::new();
        let secret_store = Platform::secret_store(&platform);
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store,
            restored_client: Mutex::new(None),
        }
    }

    /// Construct a real Core whose `Platform::secret_store` is the Swift vault.
    pub fn new_with_secret_store(store: Box<dyn IosSecretVault>) -> Self {
        let vault: Arc<dyn SecretVault + Send + Sync> =
            Arc::new(CallbackSecretVault { inner: store });
        let platform = IosFailClosedPlatform::with_secret_store(Arc::clone(&vault));
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store: vault,
            restored_client: Mutex::new(None),
        }
    }

    /// Restore an already-persisted session from the S3a vault. No password.
    ///
    /// `store_root` is the shell-owned SDK store directory. It is never echoed.
    /// This is not `matrix_restore_session` and does not attach owners or
    /// expose `Core.command`.
    pub async fn restore_persisted_session(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
    ) -> Result<SessionRestoreDto, SessionRestoreError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| restore_failed(IDENTITY_INVALID_CODE, IDENTITY_INVALID_DESCRIPTION))?;
        let root = validate_store_root(&store_root)?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        if vault
            .get(&SessionMaterialId::from_identity(&identity))
            .map_err(|_| restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION))?
            .is_none()
        {
            return Err(restore_failed(
                MATERIAL_MISSING_CODE,
                MATERIAL_MISSING_DESCRIPTION,
            ));
        }

        let store_key = store_key_for(&vault, &identity)?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let outcome = restore_session_from_vault(&client, &identity, &vault)
            .await
            .map_err(|error| match error {
                crate::app::lifecycle::LifecycleError::Vault {
                    diagnostic_id: "p3.6-session-material-missing",
                    ..
                } => restore_failed(MATERIAL_MISSING_CODE, MATERIAL_MISSING_DESCRIPTION),
                _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
            })?;

        let snapshot = SessionSnapshot {
            session_generation: 1,
            user_id: outcome.meta.user_id.clone(),
            device_id: outcome.meta.device_id.clone(),
            homeserver_url: outcome.meta.homeserver_url.clone(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;

        let mut guard = self
            .restored_client
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        *guard = Some(client);

        Ok(SessionRestoreDto {
            user_id: outcome.meta.user_id,
            device_id: outcome.meta.device_id,
            homeserver_url: outcome.meta.homeserver_url,
        })
    }
}

fn validate_store_root(store_root: &str) -> Result<&Path, SessionRestoreError> {
    let trimmed = store_root.trim();
    if trimmed.is_empty() {
        return Err(restore_failed(
            STORE_ROOT_INVALID_CODE,
            STORE_ROOT_INVALID_DESCRIPTION,
        ));
    }
    let path = Path::new(trimmed);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(restore_failed(
            STORE_ROOT_INVALID_CODE,
            STORE_ROOT_INVALID_DESCRIPTION,
        ));
    }
    Ok(path)
}

fn store_key_for(
    vault: &SecretStoreSessionVault,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let key_name = format!("{STORE_KEY_PREFIX}{}", identity.account_dir_segment());
    match vault.store.get(&key_name) {
        Ok(Some(bytes)) if bytes.len() == STORE_KEY_LEN => {
            let mut key_bytes = [0u8; STORE_KEY_LEN];
            key_bytes.copy_from_slice(&bytes);
            Ok(StoreKeyMaterial::from_bytes(key_bytes))
        }
        Ok(Some(_)) | Ok(None) => generate_and_store_key(vault, &key_name),
        Err(_) => Err(restore_failed(
            VAULT_UNAVAILABLE_CODE,
            VAULT_UNAVAILABLE_DESCRIPTION,
        )),
    }
}

fn generate_and_store_key(
    vault: &SecretStoreSessionVault,
    key_name: &str,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let key = StoreKeyMaterial::generate()
        .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
    vault
        .store
        .put(key_name, key.as_bytes().as_slice())
        .map_err(|_| restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION))?;
    Ok(key)
}

struct CallbackSecretVault {
    inner: Box<dyn IosSecretVault>,
}

impl SecretVault for CallbackSecretVault {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError> {
        self.inner
            .get(key.to_owned())
            .map_err(|_| vault_unavailable())
    }

    fn put(&self, key: &str, value: &[u8]) -> Result<(), MatrixIpcError> {
        self.inner
            .put(key.to_owned(), value.to_vec())
            .map_err(|_| vault_unavailable())
    }

    fn delete(&self, key: &str) -> Result<(), MatrixIpcError> {
        self.inner
            .delete(key.to_owned())
            .map_err(|_| vault_unavailable())
    }
}

struct SecretStoreSessionVault {
    store: Arc<dyn SecretVault + Send + Sync>,
}

impl SessionMaterialVault for SecretStoreSessionVault {
    fn get(
        &self,
        id: &SessionMaterialId,
    ) -> Result<Option<SessionMaterial>, crate::app::lifecycle::LifecycleError> {
        match self.store.get(id.account()) {
            Ok(Some(bytes)) => Ok(Some(SessionMaterial::from_sealed_blob(bytes))),
            Ok(None) => Ok(None),
            Err(_) => Err(crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }),
        }
    }

    fn set(
        &self,
        id: &SessionMaterialId,
        material: &SessionMaterial,
    ) -> Result<(), crate::app::lifecycle::LifecycleError> {
        self.store
            .put(id.account(), material.as_bytes())
            .map_err(|_| crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            })
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, crate::app::lifecycle::LifecycleError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store.delete(id.account()).map_err(|_| {
            crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }
        })?;
        Ok(existed)
    }
}

fn vault_unavailable() -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::StoreUnavailable)
        .with_diagnostic("p4-s3-secret-vault-unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::lifecycle::persist_session_material;
    use crate::transport::MatrixIpcErrorCategory;
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MemoryCallbackVault(std::sync::Arc<Mutex<HashMap<String, Vec<u8>>>>);

    impl IosSecretVault for MemoryCallbackVault {
        fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
            Ok(self.0.lock().expect("vault").get(&key).cloned())
        }

        fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
            self.0.lock().expect("vault").insert(key, value);
            Ok(())
        }

        fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
            self.0.lock().expect("vault").remove(&key);
            Ok(())
        }
    }

    fn alice() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-p4-s3b-{tag}-{nanos}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn test_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn shared_core_constructs_and_retains_the_built_in_core() {
        let shared_core = SharedCore::new();
        assert!(
            !shared_core.core.registered_commands().is_empty(),
            "P4-S2 must retain a real Core with its built-in registry"
        );
    }

    #[test]
    fn shared_core_with_secret_store_round_trips_through_the_callback() {
        let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
            HashMap::new(),
        ))));
        let shared = SharedCore::new_with_secret_store(store);
        assert!(
            !shared.core.registered_commands().is_empty(),
            "P4-S3a must still retain a real Core"
        );
    }

    #[test]
    fn callback_vault_maps_foreign_failure_to_static_store_unavailable() {
        struct FailingVault;
        impl IosSecretVault for FailingVault {
            fn get(&self, _: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
                Err(IosSecretVaultError::Unavailable {
                    code: "p4-s3-secret-vault-unavailable".to_owned(),
                    description: "The secret store is unavailable.".to_owned(),
                })
            }
            fn put(&self, _: String, _: Vec<u8>) -> Result<(), IosSecretVaultError> {
                unreachable!("put")
            }
            fn delete(&self, _: String) -> Result<(), IosSecretVaultError> {
                unreachable!("delete")
            }
        }

        let vault = CallbackSecretVault {
            inner: Box::new(FailingVault),
        };
        let error = vault.get("session").expect_err("must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::StoreUnavailable);
        assert!(!format!("{error:?}").contains("session"));
    }

    #[test]
    fn restore_without_vault_fails_closed_without_echoing_identity() {
        let shared = SharedCore::new();
        let root = temp_root("no-vault");
        let rt = test_runtime();
        let error = rt
            .block_on(shared.restore_persisted_session(
                "@alice:example.org".to_owned(),
                "https://matrix.example.org".to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("fail-closed vault cannot restore");
        let text = format!("{error:?}");
        assert!(text.contains(VAULT_UNAVAILABLE_CODE) || text.contains(MATERIAL_MISSING_CODE));
        assert!(!text.contains("@alice"));
        assert!(!text.contains("matrix.example.org"));
        assert!(!text.contains(root.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_rejects_hostile_identity_without_echo() {
        let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
            HashMap::new(),
        ))));
        let shared = SharedCore::new_with_secret_store(store);
        let root = temp_root("hostile");
        let rt = test_runtime();
        let hostile = "https://user:secret@evil.example/?password=hunter2";
        let error = rt
            .block_on(shared.restore_persisted_session(
                "not-a-user".to_owned(),
                hostile.to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("invalid identity");
        let text = format!("{error:?}{error}");
        assert!(text.contains(IDENTITY_INVALID_CODE));
        assert!(!text.contains("secret"));
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("evil.example"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_from_vault_installs_session_without_password_or_token_leak() {
        let access = "syt_s3b_access_token_value";
        let refresh = "syr_s3b_refresh_token_value";
        let identity = alice();
        let material =
            SessionMaterial::from_matrix_tokens(&identity, "DEVICEABC", access, Some(refresh))
                .unwrap();
        let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let persist_vault = SecretStoreSessionVault {
            store: Arc::new(CallbackSecretVault {
                inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
            }),
        };
        persist_session_material(&persist_vault, &identity, &material).unwrap();
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(map)));
        let root = temp_root("restore");
        let rt = test_runtime();
        let _enter = rt.enter();
        let dto = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect("restore");
        assert_eq!(dto.user_id, "@alice:example.org");
        assert_eq!(dto.device_id, "DEVICEABC");
        assert_eq!(dto.homeserver_url, "https://matrix.example.org");
        let dbg = format!("{dto:?}");
        assert!(!dbg.contains(access));
        assert!(!dbg.contains(refresh));
        assert!(!dbg.contains("password"));
        let snapshot = shared.core.session_snapshot().expect("projection");
        assert!(snapshot.is_some());
        assert!(shared.restored_client.lock().expect("client").is_some());
        drop(shared);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }
}
