//! Fail-closed iOS Platform used before the Apple shell owns a live session.
//!
//! This deliberately has no Swift callback, Keychain, APNs, Tauri, or keyring
//! dependency. P4-S2 needs only a concrete [`Platform`] so UniFFI can retain
//! a real [`Core`](crate::Core); P4-S3 designs the live iOS session separately.

use std::sync::Arc;

use super::{
    CrossSigningStatusFuture, CryptoStatusFuture, MediaConfigFuture, Platform,
    PlatformCrossSigningStatusError, PlatformCryptoStatusError, PlatformMediaConfigError,
    PlatformStatus, PlatformSyncStatusError, SecretVault, SyncStatusFuture, UnavailableSecretVault,
};
use crate::dto::NotificationCandidate;
use crate::transport::{MatrixIpcEnvelope, MatrixIpcError};

/// iOS shell placeholder that permits only safe Core construction.
///
/// Calls that need a live client return the existing static unavailable or
/// no-session errors. The typed IPC envelope sink and OS notification/status
/// sinks are intentional no-ops until the corresponding iOS shell work lands.
#[derive(Debug, Default)]
pub struct IosFailClosedPlatform;

impl Platform for IosFailClosedPlatform {
    fn emit(&self, _: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }

    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
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
