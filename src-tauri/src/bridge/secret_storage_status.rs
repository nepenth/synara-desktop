//! Strict desktop bridge for the zero-argument `matrix_secret_storage_status` command.
//!
//! Core owns registry dispatch, payload validation, and exact legacy response
//! serialization. The desktop remains the sole owner of the Matrix SDK client,
//! account-data reads, store, keys, and recovery state. This bridge accepts
//! only the strict public DTO or static Core errors and never reflects text
//! supplied by Core.

use synara_core::core_api::{
    MatrixMissingSecretResponse, MatrixSecretStorageActionResponse,
    MatrixSecretStorageStateResponse, MatrixSecretStorageStatusResponse,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory, MAX_WIRE_COUNTER};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::secret_storage::live::{
    NativeMissingSecret, NativeSecretStorageAction, NativeSecretStorageState,
    NativeSecretStorageStatus,
};

/// Map Core's closed result onto the desktop status, revalidating its
/// invariants.
fn native_secret_storage_status(
    response: MatrixSecretStorageStatusResponse,
) -> Result<NativeSecretStorageStatus, ()> {
    let state = match response.state {
        MatrixSecretStorageStateResponse::Unavailable => NativeSecretStorageState::Unavailable,
        MatrixSecretStorageStateResponse::NotSetUp => NativeSecretStorageState::NotSetUp,
        MatrixSecretStorageStateResponse::Locked => NativeSecretStorageState::Locked,
        MatrixSecretStorageStateResponse::Ready => NativeSecretStorageState::Ready,
    };
    let action = match response.action {
        MatrixSecretStorageActionResponse::BootstrapRequired => {
            NativeSecretStorageAction::BootstrapRequired
        }
        MatrixSecretStorageActionResponse::UnlockRequired => {
            NativeSecretStorageAction::UnlockRequired
        }
        MatrixSecretStorageActionResponse::None => NativeSecretStorageAction::None,
    };
    let missing_secrets = decode_missing_secrets(response.missing_secrets)?;
    let status = NativeSecretStorageStatus {
        session_generation: response.session_generation,
        state,
        exists: response.exists,
        unlocked: response.unlocked,
        default_key_set: response.default_key_set,
        passphrase_configured: response.passphrase_configured,
        bootstrap_ready: response.bootstrap_ready,
        missing_secrets,
        action,
    };
    secret_storage_status_is_valid(&status)
        .then_some(status)
        .ok_or(())
}

/// Decode only a canonical, strictly ordered subset of the four legacy public
/// labels. The Core creates this order; requiring it makes malformed/hostile
/// output fail closed rather than become a desktop status object.
fn decode_missing_secrets(
    values: Vec<MatrixMissingSecretResponse>,
) -> Result<Vec<NativeMissingSecret>, ()> {
    let mut previous = None;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let (rank, native) = match value {
            MatrixMissingSecretResponse::CrossSigningMaster => {
                (0, NativeMissingSecret::CrossSigningMaster)
            }
            MatrixMissingSecretResponse::CrossSigningSelfSigning => {
                (1, NativeMissingSecret::CrossSigningSelfSigning)
            }
            MatrixMissingSecretResponse::CrossSigningUserSigning => {
                (2, NativeMissingSecret::CrossSigningUserSigning)
            }
            MatrixMissingSecretResponse::EncryptionBackup => {
                (3, NativeMissingSecret::EncryptionBackup)
            }
        };
        if previous.is_some_and(|previous| rank <= previous) {
            return Err(());
        }
        previous = Some(rank);
        decoded.push(native);
    }
    Ok(decoded)
}

/// Forward the existing payload-free Tauri command through Core. No recovery
/// secret, key id, account-data object, SDK client/store, or raw SDK error can
/// enter this bridge.
pub(crate) async fn secret_storage_status(
    core: &Core,
) -> Result<NativeSecretStorageStatus, MatrixAuthCommandError> {
    let response = core
        .secret_storage_status()
        .await
        .map_err(map_secret_storage_status_core_error)?;
    decode_secret_storage_status_response(response)
}

/// Validate complete Core response metadata plus the exact legacy DTO before
/// constructing the prior desktop response type.
fn decode_secret_storage_status_response(
    response: MatrixSecretStorageStatusResponse,
) -> Result<NativeSecretStorageStatus, MatrixAuthCommandError> {
    native_secret_storage_status(response).map_err(|_| secret_storage_status_response_error())
}

/// Revalidate every relationship that the legacy desktop status owner emits.
fn secret_storage_status_is_valid(status: &NativeSecretStorageStatus) -> bool {
    status.session_generation <= MAX_WIRE_COUNTER
        && matches!(
            (status.state, status.unlocked, status.action),
            (
                NativeSecretStorageState::Unavailable,
                false,
                NativeSecretStorageAction::UnlockRequired,
            ) | (
                NativeSecretStorageState::NotSetUp,
                false,
                NativeSecretStorageAction::BootstrapRequired,
            ) | (
                NativeSecretStorageState::Locked,
                false,
                NativeSecretStorageAction::UnlockRequired,
            ) | (
                NativeSecretStorageState::Ready,
                true,
                NativeSecretStorageAction::None,
            )
        )
        && status
            .missing_secrets
            .windows(2)
            .all(|pair| missing_secret_rank(pair[0]) < missing_secret_rank(pair[1]))
}

fn missing_secret_rank(value: NativeMissingSecret) -> u8 {
    match value {
        NativeMissingSecret::CrossSigningMaster => 0,
        NativeMissingSecret::CrossSigningSelfSigning => 1,
        NativeMissingSecret::CrossSigningUserSigning => 2,
        NativeMissingSecret::EncryptionBackup => 3,
    }
}

/// Restore only the exact closed Core category/diagnostic pairs corresponding
/// to the old desktop errors. Unknown malformed values become one static bridge
/// failure and never reflect Core text.
fn map_secret_storage_status_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match (error.category, error.diagnostic_id.as_deref()) {
        (MatrixIpcErrorCategory::Forbidden, Some("v-crypto.4-secret-storage-requires-session")) => {
            secret_storage_requires_session_error()
        }
        (MatrixIpcErrorCategory::RecoveryFailure, Some("v-crypto.4-status-default-key-failed")) => {
            secret_storage_default_key_error()
        }
        (MatrixIpcErrorCategory::RecoveryFailure, Some("v-crypto.4-status-key-info-failed")) => {
            secret_storage_key_info_error()
        }
        (
            MatrixIpcErrorCategory::RecoveryFailure,
            Some("v-crypto.4-status-secret-check-failed"),
        ) => secret_storage_secret_check_error(),
        _ => secret_storage_status_core_error(),
    }
}

fn secret_storage_requires_session_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Forbidden",
        "No native Matrix session is active.",
        "v-crypto.4-secret-storage-requires-session",
    )
}

fn secret_storage_default_key_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Recovery",
        "Native secret storage status is unavailable.",
        "v-crypto.4-status-default-key-failed",
    )
}

fn secret_storage_key_info_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Recovery",
        "Native secret storage status is unavailable.",
        "v-crypto.4-status-key-info-failed",
    )
}

fn secret_storage_secret_check_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Recovery",
        "Native secret storage status is unavailable.",
        "v-crypto.4-status-secret-check-failed",
    )
}

fn secret_storage_status_core_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Recovery",
        "Native secret storage status is unavailable.",
        "snc-p2-secret-storage-status-core-failed",
    )
}

fn secret_storage_status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Recovery",
        "Native secret storage status is unavailable.",
        "snc-p2-secret-storage-status-response-invalid",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        Platform, PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig,
        PlatformSecretStorageStatusError, PlatformStatus, PlatformSyncStatus, SecretVault,
        UnavailableSecretVault,
    };
    use synara_core::transport::MatrixIpcEnvelope;

    use super::*;

    struct TestPlatform;

    impl Platform for TestPlatform {
        fn emit(&self, _envelope: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }

        fn http_user_agent(&self) -> String {
            "Synara-Desktop-Secret-Storage-Bridge-Test/1.0".to_owned()
        }

        fn sync_status(&self) -> synara_core::platform::SyncStatusFuture<'_> {
            Box::pin(async {
                Ok(PlatformSyncStatus::new(
                    synara_core::app::sync::SyncReadiness::Unconfigured,
                    0,
                    false,
                    None,
                    None,
                )
                .expect("unconfigured status is a valid closed projection"))
            })
        }

        fn crypto_status(&self) -> synara_core::platform::CryptoStatusFuture<'_> {
            Box::pin(async {
                Ok(PlatformCryptoStatus::new(
                    0,
                    false,
                    PlatformCryptoCrossSigningState::Unavailable,
                )
                .expect("unavailable crypto status is a valid closed projection"))
            })
        }

        fn cross_signing_status(&self) -> synara_core::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async {
                Err(synara_core::platform::PlatformCrossSigningStatusError::NoSession)
            })
        }

        fn secret_storage_status(&self) -> synara_core::platform::SecretStorageStatusFuture<'_> {
            Box::pin(async { Err(PlatformSecretStorageStatusError::NoSession) })
        }

        fn media_config(&self) -> synara_core::platform::MediaConfigFuture<'_> {
            Box::pin(async {
                Ok(PlatformMediaConfig::new(0)
                    .expect("zero is a valid closed media-config projection"))
            })
        }

        fn notify(&self, _candidate: NotificationCandidate) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    #[test]
    fn bridge_restores_every_legacy_static_core_error_pair() {
        for (category, diagnostic_id, code, message) in [
            (
                MatrixIpcErrorCategory::Forbidden,
                "v-crypto.4-secret-storage-requires-session",
                "Forbidden",
                "No native Matrix session is active.",
            ),
            (
                MatrixIpcErrorCategory::RecoveryFailure,
                "v-crypto.4-status-default-key-failed",
                "Recovery",
                "Native secret storage status is unavailable.",
            ),
            (
                MatrixIpcErrorCategory::RecoveryFailure,
                "v-crypto.4-status-key-info-failed",
                "Recovery",
                "Native secret storage status is unavailable.",
            ),
            (
                MatrixIpcErrorCategory::RecoveryFailure,
                "v-crypto.4-status-secret-check-failed",
                "Recovery",
                "Native secret storage status is unavailable.",
            ),
        ] {
            let error = map_secret_storage_status_core_error(
                MatrixIpcError::new(category).with_diagnostic(diagnostic_id),
            );
            assert_eq!(error.code, code);
            assert_eq!(error.message, message);
            assert_eq!(error.diagnostic_id, diagnostic_id);
        }
        for error in [
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-crypto.4-status-default-key-failed"),
            MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("not-a-legacy-diagnostic"),
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("p2-secret-storage-status-invalid-platform-projection"),
        ] {
            assert_eq!(
                map_secret_storage_status_core_error(error).diagnostic_id,
                "snc-p2-secret-storage-status-core-failed"
            );
        }
    }

    #[tokio::test]
    async fn secret_storage_status_without_session_maps_to_the_static_error() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = secret_storage_status(&core).await.expect_err("no session");
        assert_eq!(error.code, "Forbidden");
    }
}
