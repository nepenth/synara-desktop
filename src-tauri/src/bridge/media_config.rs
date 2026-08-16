//! Strict desktop bridge for the zero-argument `matrix_media_config` command.
//!
//! Core owns command registration, envelope validation, and the exact public
//! object spelling. Desktop remains the owner of the Matrix SDK client/session,
//! cache, and store. This bridge accepts only the one bounded legacy payload and
//! maps every Core failure to a static desktop error without reflecting Core or
//! SDK text.

use serde::Deserialize;
use synara_core::transport::{
    CommandEnvelope, CommandResponseEnvelope, MatrixIpcError, MatrixIpcErrorCategory,
    MAX_WIRE_COUNTER,
};
use synara_core::Core;

use crate::matrix::auth::product::{MatrixAuthCommandError, MatrixMediaConfigResult};

const MEDIA_CONFIG_COMMAND: &str = "matrix_media_config";
/// `matrix_media_config` is an observation without a renderer session payload.
const READ_ONLY_SESSION_GENERATION: u64 = 0;

/// Strict, bridge-local decoder for Core's one-key legacy media configuration
/// object. There are no defaults and unknown fields are rejected so hostile
/// text cannot become part of a desktop DTO or error.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaConfigWireResponse {
    #[serde(rename = "m.upload.size")]
    upload_size: u64,
}

/// Route the existing zero-argument Tauri command through the Core registry.
/// The envelope uses the same neutral, JSON-safe read-only generation as the
/// other desktop observations; no renderer input crosses this boundary.
pub(crate) async fn media_config(
    core: &Core,
) -> Result<MatrixMediaConfigResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: MEDIA_CONFIG_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
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
    response: CommandResponseEnvelope,
) -> Result<MatrixMediaConfigResult, MatrixAuthCommandError> {
    if response.command != MEDIA_CONFIG_COMMAND
        || response.session_generation != READ_ONLY_SESSION_GENERATION
        || response.request_id.is_some()
    {
        return Err(media_config_response_error());
    }
    let response: MediaConfigWireResponse =
        serde_json::from_value(response.payload).map_err(|_| media_config_response_error())?;
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
    use std::sync::{Arc, Mutex};

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        Platform, PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig,
        PlatformStatus, PlatformSyncStatus, SecretVault, UnavailableSecretVault,
    };
    use synara_core::transport::{CommandFuture, CommandRegistry, MatrixIpcEnvelope};

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

    fn core_returning(
        payload: serde_json::Value,
        forwarded: Arc<Mutex<Vec<CommandEnvelope>>>,
    ) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                MEDIA_CONFIG_COMMAND,
                move |_state, request| -> CommandFuture {
                    forwarded.lock().expect("test capture lock").push(request);
                    let payload = payload.clone();
                    Box::pin(async move { Ok(payload) })
                },
            )
            .expect("media config is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    fn core_failing(error: MatrixIpcError) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                MEDIA_CONFIG_COMMAND,
                move |_state, _request| -> CommandFuture {
                    let error = error.clone();
                    Box::pin(async move { Err(error) })
                },
            )
            .expect("media config is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    fn response(payload: serde_json::Value) -> CommandResponseEnvelope {
        CommandResponseEnvelope {
            command: MEDIA_CONFIG_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        }
    }

    #[test]
    fn media_config_envelope_is_payload_free_and_uses_a_valid_neutral_generation() {
        let envelope = CommandEnvelope {
            command: MEDIA_CONFIG_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        };
        assert!(envelope.validate().is_ok());
    }

    #[tokio::test]
    async fn media_config_bridge_forwards_exact_envelope_and_legacy_wire_object() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = serde_json::json!({ "m.upload.size": MAX_WIRE_COUNTER });
        let result = media_config(&core_returning(payload.clone(), Arc::clone(&forwarded)))
            .await
            .expect("known Core media config remains the desktop DTO");

        assert_eq!(serde_json::to_value(result).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: MEDIA_CONFIG_COMMAND.to_owned(),
                session_generation: READ_ONLY_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            }]
        );
    }

    #[test]
    fn strict_media_config_decoder_rejects_unknown_hostile_malformed_and_unsafe_responses() {
        let private_text = "https://private.example token=secret password=secret key=secret";
        for payload in [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!({ "m.upload.size": private_text }),
            serde_json::json!({ "m.upload.size": 1, "private": private_text }),
            serde_json::json!({ "m.upload.size": MAX_WIRE_COUNTER + 1 }),
            serde_json::json!({ "m.upload.size": -1 }),
        ] {
            let error = decode_media_config_response(response(payload))
                .expect_err("malformed Core media response must fail closed");
            assert_eq!(
                error.diagnostic_id,
                "snc-p3-5-media-config-response-invalid"
            );
            let serialized =
                serde_json::to_string(&error).expect("static desktop error serializes");
            for forbidden in ["private.example", "token", "secret", "password", "key"] {
                assert!(
                    !serialized.contains(forbidden),
                    "strict decoder must not reflect hostile Core text: {forbidden}"
                );
            }
        }

        let wrong_metadata = CommandResponseEnvelope {
            command: "matrix_media_download".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "m.upload.size": 1 }),
        };
        assert_eq!(
            decode_media_config_response(wrong_metadata)
                .expect_err("wrong Core response metadata is not accepted")
                .diagnostic_id,
            "snc-p3-5-media-config-response-invalid"
        );
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
    async fn media_config_bridge_errors_are_static_and_never_reflect_core_text() {
        let private_text = "https://private.example token=secret password=secret key=secret";
        let core_error = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some(private_text.to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let error = media_config(&core_failing(core_error))
            .await
            .expect_err("Core failure maps to the existing static media error");
        assert_eq!(error.code, "Unknown");
        assert_eq!(error.diagnostic_id, "v-send.r-media-config-sdk-failed");
        let serialized = serde_json::to_string(&error).expect("static desktop error serializes");
        for forbidden in ["private.example", "token", "secret", "password", "key"] {
            assert!(
                !serialized.contains(forbidden),
                "bridge error must not reflect private Core data: {forbidden}"
            );
        }
    }
}
