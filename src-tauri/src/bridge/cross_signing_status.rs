//! Strict desktop bridge for `matrix_cross_signing_status`.
//!
//! This is a read-observation adapter only. Core owns registration, envelope
//! validation, the exact legacy truth table, and public serialization. Desktop
//! retains the Matrix SDK client/crypto/store/network and accepts only the
//! validated legacy DTO or a static error here.

use synara_core::core_api::{
    MatrixCrossSigningBootstrapResponse, MatrixCrossSigningKeyPublicationResponse,
    MatrixCrossSigningPrivateIdentityResponse, MatrixCrossSigningReadinessResponse,
    MatrixCrossSigningStatusResponse, MatrixOwnIdentityVerificationResponse,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory, MAX_WIRE_COUNTER};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::cross_signing::live::{
    NativeCrossSigningBootstrap, NativeCrossSigningKeyPublication,
    NativeCrossSigningPrivateIdentity, NativeCrossSigningReadiness, NativeCrossSigningStatus,
    NativeOwnIdentityVerification,
};

/// Map Core's closed result onto the desktop status, revalidating its
/// invariants.
fn native_cross_signing_status(
    response: MatrixCrossSigningStatusResponse,
) -> Result<NativeCrossSigningStatus, ()> {
    let readiness = match response.readiness {
        MatrixCrossSigningReadinessResponse::Unavailable => {
            NativeCrossSigningReadiness::Unavailable
        }
        MatrixCrossSigningReadinessResponse::SetupRequired => {
            NativeCrossSigningReadiness::SetupRequired
        }
        MatrixCrossSigningReadinessResponse::RecoveryRequired => {
            NativeCrossSigningReadiness::RecoveryRequired
        }
        MatrixCrossSigningReadinessResponse::VerificationRequired => {
            NativeCrossSigningReadiness::VerificationRequired
        }
        MatrixCrossSigningReadinessResponse::Ready => NativeCrossSigningReadiness::Ready,
    };
    let publication = match response.master_signing {
        MatrixCrossSigningKeyPublicationResponse::Missing => {
            NativeCrossSigningKeyPublication::Missing
        }
        MatrixCrossSigningKeyPublicationResponse::Published => {
            NativeCrossSigningKeyPublication::Published
        }
    };
    let self_signing = match response.self_signing {
        MatrixCrossSigningKeyPublicationResponse::Missing => {
            NativeCrossSigningKeyPublication::Missing
        }
        MatrixCrossSigningKeyPublicationResponse::Published => {
            NativeCrossSigningKeyPublication::Published
        }
    };
    let user_signing = match response.user_signing {
        MatrixCrossSigningKeyPublicationResponse::Missing => {
            NativeCrossSigningKeyPublication::Missing
        }
        MatrixCrossSigningKeyPublicationResponse::Published => {
            NativeCrossSigningKeyPublication::Published
        }
    };
    let private_identity = match response.private_identity {
        MatrixCrossSigningPrivateIdentityResponse::Missing => {
            NativeCrossSigningPrivateIdentity::Missing
        }
        MatrixCrossSigningPrivateIdentityResponse::Partial => {
            NativeCrossSigningPrivateIdentity::Partial
        }
        MatrixCrossSigningPrivateIdentityResponse::Complete => {
            NativeCrossSigningPrivateIdentity::Complete
        }
    };
    let own_identity_verification = match response.own_identity_verification {
        MatrixOwnIdentityVerificationResponse::Missing => NativeOwnIdentityVerification::Missing,
        MatrixOwnIdentityVerificationResponse::Unverified => {
            NativeOwnIdentityVerification::Unverified
        }
        MatrixOwnIdentityVerificationResponse::Verified => NativeOwnIdentityVerification::Verified,
    };
    let bootstrap = match response.bootstrap {
        MatrixCrossSigningBootstrapResponse::Needed => NativeCrossSigningBootstrap::Needed,
        MatrixCrossSigningBootstrapResponse::NotNeeded => NativeCrossSigningBootstrap::NotNeeded,
    };
    let status = NativeCrossSigningStatus {
        session_generation: response.session_generation,
        readiness,
        master_signing: publication,
        self_signing,
        user_signing,
        private_identity,
        own_identity_verification,
        bootstrap,
    };
    cross_signing_status_is_valid(&status)
        .then_some(status)
        .ok_or(())
}

/// Forward the existing zero-argument Tauri command through Core. No SDK
/// identity, user id, key, secret, client/store, or raw error is accepted by
/// this bridge.
pub(crate) async fn cross_signing_status(
    core: &Core,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    let response = core
        .cross_signing_status()
        .await
        .map_err(map_cross_signing_status_core_error)?;
    decode_cross_signing_status_response(response)
}

/// Verify Core's complete response envelope and exact legacy truth table. Even
/// though Core constructs the normal response, this stays fail-closed against a
/// future malformed registry/transport implementation and returns no parse text.
fn decode_cross_signing_status_response(
    response: MatrixCrossSigningStatusResponse,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    native_cross_signing_status(response).map_err(|_| cross_signing_status_response_error())
}

/// Revalidate every legacy output relationship locally after strict decode.
/// `recovery_required` is only a closed status label; this function never
/// invokes a recovery, setup, or verification operation.
fn cross_signing_status_is_valid(status: &NativeCrossSigningStatus) -> bool {
    if status.session_generation > MAX_WIRE_COUNTER
        || status.master_signing != status.self_signing
        || status.master_signing != status.user_signing
    {
        return false;
    }
    let identity_is_consistent = matches!(
        (status.master_signing, status.own_identity_verification),
        (
            NativeCrossSigningKeyPublication::Missing,
            NativeOwnIdentityVerification::Missing
        ) | (
            NativeCrossSigningKeyPublication::Published,
            NativeOwnIdentityVerification::Unverified | NativeOwnIdentityVerification::Verified
        )
    );
    identity_is_consistent
        && matches!(
            (
                status.readiness,
                status.private_identity,
                status.own_identity_verification,
                status.bootstrap,
            ),
            (
                NativeCrossSigningReadiness::Unavailable,
                NativeCrossSigningPrivateIdentity::Missing,
                _,
                NativeCrossSigningBootstrap::NotNeeded,
            ) | (
                NativeCrossSigningReadiness::SetupRequired,
                NativeCrossSigningPrivateIdentity::Missing
                    | NativeCrossSigningPrivateIdentity::Partial
                    | NativeCrossSigningPrivateIdentity::Complete,
                NativeOwnIdentityVerification::Missing,
                NativeCrossSigningBootstrap::Needed,
            ) | (
                NativeCrossSigningReadiness::RecoveryRequired,
                NativeCrossSigningPrivateIdentity::Missing
                    | NativeCrossSigningPrivateIdentity::Partial,
                NativeOwnIdentityVerification::Unverified | NativeOwnIdentityVerification::Verified,
                NativeCrossSigningBootstrap::NotNeeded,
            ) | (
                NativeCrossSigningReadiness::VerificationRequired,
                NativeCrossSigningPrivateIdentity::Complete,
                NativeOwnIdentityVerification::Unverified,
                NativeCrossSigningBootstrap::NotNeeded,
            ) | (
                NativeCrossSigningReadiness::Ready,
                NativeCrossSigningPrivateIdentity::Complete,
                NativeOwnIdentityVerification::Verified,
                NativeCrossSigningBootstrap::NotNeeded,
            )
        )
}

/// Restore only the exact static Core category/diagnostic pairs that represent
/// the established desktop errors. Every other Core value, including hostile
/// text, unknown diagnostics/categories, request ids, or raw error messages,
/// maps to one fixed SNC-P3.6 error.
fn map_cross_signing_status_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match (error.category, error.diagnostic_id.as_deref()) {
        (MatrixIpcErrorCategory::Forbidden, Some("v-crypto.2-cross-signing-requires-session")) => {
            cross_signing_requires_session_error()
        }
        (MatrixIpcErrorCategory::Forbidden, Some("v-crypto.2-cross-signing-user-missing")) => {
            cross_signing_user_missing_error()
        }
        (
            MatrixIpcErrorCategory::Unknown,
            Some("v-crypto.2-cross-signing-identity-query-failed"),
        ) => cross_signing_identity_query_error(),
        _ => cross_signing_status_core_error(),
    }
}

fn cross_signing_requires_session_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Forbidden",
        "No native Matrix session is active.",
        "v-crypto.2-cross-signing-requires-session",
    )
}

fn cross_signing_user_missing_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Forbidden",
        "No native Matrix session is active.",
        "v-crypto.2-cross-signing-user-missing",
    )
}

fn cross_signing_identity_query_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native cross-signing status is unavailable.",
        "v-crypto.2-cross-signing-identity-query-failed",
    )
}

fn cross_signing_status_core_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native cross-signing status is unavailable.",
        "snc-p3-6-cross-signing-status-core-failed",
    )
}

fn cross_signing_status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native cross-signing status is unavailable.",
        "snc-p3-6-cross-signing-status-response-invalid",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        CrossSigningStatusFuture, Platform, PlatformCrossSigningStatusError,
        PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig, PlatformStatus,
        PlatformSyncStatus, SecretVault, UnavailableSecretVault,
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
            "Synara-Desktop-Cross-Signing-Bridge-Test/1.0".to_owned()
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
                .expect("unconfigured status is a valid string-free projection"))
            })
        }

        fn crypto_status(&self) -> synara_core::platform::CryptoStatusFuture<'_> {
            Box::pin(async {
                Ok(PlatformCryptoStatus::new(
                    0,
                    false,
                    PlatformCryptoCrossSigningState::Unavailable,
                )
                .expect("unavailable is a valid string-free crypto projection"))
            })
        }

        fn cross_signing_status(&self) -> CrossSigningStatusFuture<'_> {
            Box::pin(async { Err(PlatformCrossSigningStatusError::NoSession) })
        }

        fn media_config(&self) -> synara_core::platform::MediaConfigFuture<'_> {
            Box::pin(async {
                Ok(PlatformMediaConfig::new(0).expect("zero is a valid closed media projection"))
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
    fn bridge_restores_only_exact_legacy_core_error_pairs() {
        for (category, diagnostic_id, code) in [
            (
                MatrixIpcErrorCategory::Forbidden,
                "v-crypto.2-cross-signing-requires-session",
                "Forbidden",
            ),
            (
                MatrixIpcErrorCategory::Forbidden,
                "v-crypto.2-cross-signing-user-missing",
                "Forbidden",
            ),
            (
                MatrixIpcErrorCategory::Unknown,
                "v-crypto.2-cross-signing-identity-query-failed",
                "Unknown",
            ),
        ] {
            let error = map_cross_signing_status_core_error(
                MatrixIpcError::new(category).with_diagnostic(diagnostic_id),
            );
            assert_eq!(error.code, code);
            assert_eq!(error.diagnostic_id, diagnostic_id);
        }
        for error in [
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-crypto.2-cross-signing-user-missing"),
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-crypto.2-cross-signing-requires-session"),
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("p2-cross-signing-status-unsafe-session-generation"),
        ] {
            assert_eq!(
                map_cross_signing_status_core_error(error).diagnostic_id,
                "snc-p3-6-cross-signing-status-core-failed"
            );
        }
    }

    #[tokio::test]
    async fn cross_signing_status_without_session_maps_to_the_static_error() {
        let core = Core::new(Arc::new(TestPlatform));
        assert!(cross_signing_status(&core).await.is_err());
    }
}
