//! Fail-closed iOS Platform used before the Apple shell owns a live session.
//!
//! P4-S2 constructed this with [`UnavailableSecretVault`]. P4-S3a may install
//! a Swift-owned [`SecretVault`] callback. There is still no live Client,
//! command surface, password, APNs, Tauri, or `keyring` crate.

use std::sync::Arc;

use super::{
    CrossSigningStatusFuture, CryptoStatusFuture, MediaConfigFuture, Platform,
    PlatformCrossSigningStatusError, PlatformCryptoStatusError, PlatformMediaConfigError,
    PlatformStatus, PlatformSyncStatusError, SecretVault, SyncStatusFuture, UnavailableSecretVault,
};
use crate::dto::NotificationCandidate;
use crate::transport::{MatrixIpcEnvelope, MatrixIpcError};

/// iOS shell placeholder that permits safe Core construction.
///
/// Calls that need a live client return the existing static unavailable or
/// no-session errors. The typed IPC envelope sink and OS notification/status
/// sinks are intentional no-ops until the corresponding iOS shell work lands.
pub struct IosFailClosedPlatform {
    vault: Arc<dyn SecretVault + Send + Sync>,
}

impl Default for IosFailClosedPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl IosFailClosedPlatform {
    pub fn new() -> Self {
        Self {
            vault: Arc::new(UnavailableSecretVault),
        }
    }

    pub fn with_secret_store(vault: Arc<dyn SecretVault + Send + Sync>) -> Self {
        Self { vault }
    }
}

impl Platform for IosFailClosedPlatform {
    fn emit(&self, _: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }

    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::clone(&self.vault)
    }

    fn http_user_agent(&self) -> String {
        "Synara-iOS-Core/1.0".to_owned()
    }

    fn sync_status(&self) -> SyncStatusFuture<'_> {
        Box::pin(async { Err(PlatformSyncStatusError::Unavailable) })
    }

    fn crypto_status(&self) -> CryptoStatusFuture<'_> {
        Box::pin(async { Err(PlatformCryptoStatusError::InvalidSnapshot) })
    }

    fn cross_signing_status(&self) -> CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(PlatformCrossSigningStatusError::NoSession) })
    }

    fn media_config(&self) -> MediaConfigFuture<'_> {
        Box::pin(async { Err(PlatformMediaConfigError::NoSession) })
    }

    fn notify(&self, _: NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }

    fn set_badge(&self, _: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }

    fn status(&self, _: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MatrixIpcErrorCategory;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MemoryVault(Mutex<HashMap<String, Vec<u8>>>);

    impl SecretVault for MemoryVault {
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError> {
            Ok(self.0.lock().expect("vault").get(key).cloned())
        }

        fn put(&self, key: &str, value: &[u8]) -> Result<(), MatrixIpcError> {
            self.0
                .lock()
                .expect("vault")
                .insert(key.to_owned(), value.to_vec());
            Ok(())
        }

        fn delete(&self, key: &str) -> Result<(), MatrixIpcError> {
            self.0.lock().expect("vault").remove(key);
            Ok(())
        }
    }

    #[test]
    fn default_platform_vault_is_unavailable() {
        let platform = IosFailClosedPlatform::new();
        let error = platform
            .secret_store()
            .get("session")
            .expect_err("P4-S2 constructor stays fail-closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::StoreUnavailable);
    }

    #[test]
    fn installed_vault_round_trips_bytes_without_live_client() {
        let platform = IosFailClosedPlatform::with_secret_store(Arc::new(MemoryVault(Mutex::new(
            HashMap::new(),
        ))));
        let store = platform.secret_store();
        store.put("session", b"opaque").expect("put");
        assert_eq!(store.get("session").expect("get"), Some(b"opaque".to_vec()));
        store.delete("session").expect("delete");
        assert_eq!(store.get("session").expect("missing"), None);
        store.delete("session").expect("idempotent delete");
    }
}
