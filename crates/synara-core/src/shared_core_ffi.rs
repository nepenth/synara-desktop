//! UniFFI construction facade for the shared Core.
//!
//! P4-S2 exposed only `SharedCore::new` with a fail-closed vault. P4-S3a adds
//! `new_with_secret_store` so Swift can install a Keychain-backed
//! [`SecretVault`]. This still exposes no command, session, attach, password,
//! or APNs surface.

use std::sync::Arc;

use crate::core::Core;
use crate::platform::{IosFailClosedPlatform, SecretVault};
use crate::transport::{MatrixIpcError, MatrixIpcErrorCategory};

/// Static fail-closed vault error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IosSecretVaultError {
    Unavailable { code: String, description: String },
}

/// Swift-owned key/value secret store described by the existing UDL callback.
///
/// UniFFI UDL mode generates glue only; the trait itself must live in Rust.
pub trait IosSecretVault: Send + Sync {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError>;
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError>;
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError>;
}

impl std::fmt::Display for IosSecretVaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IosSecretVaultError {}

/// Retained shared Core for the iOS UniFFI boundary.
pub struct SharedCore {
    core: Core,
}

impl SharedCore {
    /// Construct a real Core with the fail-closed iOS Platform.
    pub fn new() -> Self {
        Self {
            core: Core::new(Arc::new(IosFailClosedPlatform::new())),
        }
    }

    /// Construct a real Core whose `Platform::secret_store` is the Swift vault.
    pub fn new_with_secret_store(store: Box<dyn IosSecretVault>) -> Self {
        let vault = Arc::new(CallbackSecretVault { inner: store });
        Self {
            core: Core::new(Arc::new(IosFailClosedPlatform::with_secret_store(vault))),
        }
    }
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

fn vault_unavailable() -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::StoreUnavailable)
        .with_diagnostic("p4-s3-secret-vault-unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MatrixIpcErrorCategory;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MemoryCallbackVault(Mutex<HashMap<String, Vec<u8>>>);

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
        let store = Box::new(MemoryCallbackVault(Mutex::new(HashMap::new())));
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
}
