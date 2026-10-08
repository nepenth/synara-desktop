//! Strict desktop bridge for the zero-argument `matrix_media_config` command.
//!
//! Core owns command registration, envelope validation, and the exact public
//! object spelling. Desktop remains the owner of the Matrix SDK client/session,
//! cache, and store. This bridge accepts only the one bounded legacy payload and
//! maps every Core failure to a static desktop error without reflecting Core or
//! SDK text.

use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory, MAX_WIRE_COUNTER};
use synara_core::Core;

use crate::matrix::auth::product::{MatrixAuthCommandError, MatrixMediaConfigResult};

/// Route the existing zero-argument Tauri command through the Core registry.
/// The envelope uses the same neutral, JSON-safe read-only generation as the
/// other desktop observations; no renderer input crosses this boundary.
pub(crate) async fn media_config(
    core: &Core,
) -> Result<MatrixMediaConfigResult, MatrixAuthCommandError> {
    let response = core
        .media_config()
        .await
        .map_err(map_media_config_core_error)?;
    decode_media_config_response(response)
}

/// Decode and validate the complete response metadata and exact legacy DTO.
///
/// Core normally constructs the response envelope itself, but this validation
/// keeps the bridge fail-closed if a future registry/transport implementation
/// becomes malformed. It never puts a parsed Core value or parsing text in the
/// returned desktop error.
fn decode_media_config_response(
    response: synara_core::core_api::MatrixMediaConfigResponse,
) -> Result<MatrixMediaConfigResult, MatrixAuthCommandError> {
    if response.upload_size > MAX_WIRE_COUNTER {
        return Err(media_config_response_error());
    }
    Ok(MatrixMediaConfigResult {
        upload_size: response.upload_size,
    })
}

/// Restore the three existing desktop media-config failures from only Core's
/// closed category. Core's platform seam never supplies a raw SDK error,
/// homeserver URL, credential, key, or `MatrixIpcError` to this bridge.
fn map_media_config_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => media_config_no_session_error(),
        MatrixIpcErrorCategory::Unknown => media_config_load_failure_error(),
        MatrixIpcErrorCategory::MediaTooLarge => media_config_unsafe_size_error(),
        _ => media_config_core_error(),
    }
}

fn media_config_no_session_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Forbidden",
        "No native Matrix session is active.",
        "d0.3-timeline-requires-session",
    )
}

fn media_config_load_failure_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native media operation is unavailable.",
        "v-send.r-media-config-sdk-failed",
    )
}

fn media_config_unsafe_size_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native media operation is unavailable.",
        "v-send.r-media-config-unsafe-size",
    )
}

fn media_config_core_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native media operation is unavailable.",
        "snc-p3-5-media-config-core-failed",
    )
}

fn media_config_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native media operation is unavailable.",
        "snc-p3-5-media-config-response-invalid",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        Platform, PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig,
        PlatformStatus, PlatformSyncStatus, SecretVault, UnavailableSecretVault,
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
            "Synara-Desktop-Media-Bridge-Test/1.0".to_owned()
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

        fn cross_signing_status(&self) -> synara_core::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async {
                Err(synara_core::platform::PlatformCrossSigningStatusError::NoSession)
            })
        }

        fn media_config(&self) -> synara_core::platform::MediaConfigFuture<'_> {
            Box::pin(async {
                Ok(PlatformMediaConfig::new(16 * 1024 * 1024)
                    .expect("normal media limit is a valid closed projection"))
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
    fn media_config_core_errors_restore_legacy_static_category_and_diagnostics() {
        for (category, expected_code, expected_diagnostic) in [
            (
                MatrixIpcErrorCategory::Forbidden,
                "Forbidden",
                "d0.3-timeline-requires-session",
            ),
            (
                MatrixIpcErrorCategory::Unknown,
                "Unknown",
                "v-send.r-media-config-sdk-failed",
            ),
            (
                MatrixIpcErrorCategory::MediaTooLarge,
                "Unknown",
                "v-send.r-media-config-unsafe-size",
            ),
        ] {
            let error = map_media_config_core_error(MatrixIpcError::new(category));
            assert_eq!(error.code, expected_code);
            assert_eq!(error.diagnostic_id, expected_diagnostic);
        }
    }

    #[tokio::test]
    async fn media_config_bridge_reads_the_platform_limit_through_typed_core() {
        let core = Core::new(Arc::new(TestPlatform));
        let result = media_config(&core).await.expect("platform media config");
        assert_eq!(result.upload_size, 16 * 1024 * 1024);
    }
}
