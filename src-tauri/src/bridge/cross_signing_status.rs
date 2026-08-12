//! Strict desktop bridge for `matrix_cross_signing_status`.
//!
//! This is a read-observation adapter only. Core owns registration, envelope
//! validation, the exact legacy truth table, and public serialization. Desktop
//! retains the Matrix SDK client/crypto/store/network and accepts only the
//! validated legacy DTO or a static error here.

use serde::Deserialize;
use synara_core::transport::{
    CommandEnvelope, CommandResponseEnvelope, MatrixIpcError, MatrixIpcErrorCategory,
    MAX_WIRE_COUNTER,
};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::cross_signing::live::{
    NativeCrossSigningBootstrap, NativeCrossSigningKeyPublication,
    NativeCrossSigningPrivateIdentity, NativeCrossSigningReadiness, NativeCrossSigningStatus,
    NativeOwnIdentityVerification,
};

const CROSS_SIGNING_STATUS_COMMAND: &str = "matrix_cross_signing_status";
/// This status command has no renderer payload or session-generation input.
const READ_ONLY_SESSION_GENERATION: u64 = 0;

/// Strict bridge-local decoder for Core's exact legacy camel-case DTO. No
/// defaults or extension fields are accepted, preventing unknown Core text from
/// becoming a desktop DTO or diagnostic.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CrossSigningStatusWireResponse {
    session_generation: u64,
    readiness: CrossSigningReadinessWire,
    master_signing: CrossSigningKeyPublicationWire,
    self_signing: CrossSigningKeyPublicationWire,
    user_signing: CrossSigningKeyPublicationWire,
    private_identity: CrossSigningPrivateIdentityWire,
    own_identity_verification: OwnIdentityVerificationWire,
    bootstrap: CrossSigningBootstrapWire,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CrossSigningReadinessWire {
    Unavailable,
    SetupRequired,
    RecoveryRequired,
    VerificationRequired,
    Ready,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CrossSigningKeyPublicationWire {
    Missing,
    Published,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CrossSigningPrivateIdentityWire {
    Missing,
    Partial,
    Complete,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OwnIdentityVerificationWire {
    Missing,
    Unverified,
    Verified,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CrossSigningBootstrapWire {
    Needed,
    NotNeeded,
}

impl TryFrom<CrossSigningStatusWireResponse> for NativeCrossSigningStatus {
    type Error = ();

    fn try_from(response: CrossSigningStatusWireResponse) -> Result<Self, Self::Error> {
        let readiness = match response.readiness {
            CrossSigningReadinessWire::Unavailable => NativeCrossSigningReadiness::Unavailable,
            CrossSigningReadinessWire::SetupRequired => NativeCrossSigningReadiness::SetupRequired,
            CrossSigningReadinessWire::RecoveryRequired => {
                NativeCrossSigningReadiness::RecoveryRequired
            }
            CrossSigningReadinessWire::VerificationRequired => {
                NativeCrossSigningReadiness::VerificationRequired
            }
            CrossSigningReadinessWire::Ready => NativeCrossSigningReadiness::Ready,
        };
        let publication = match response.master_signing {
            CrossSigningKeyPublicationWire::Missing => NativeCrossSigningKeyPublication::Missing,
            CrossSigningKeyPublicationWire::Published => {
                NativeCrossSigningKeyPublication::Published
            }
        };
        let self_signing = match response.self_signing {
            CrossSigningKeyPublicationWire::Missing => NativeCrossSigningKeyPublication::Missing,
            CrossSigningKeyPublicationWire::Published => {
                NativeCrossSigningKeyPublication::Published
            }
        };
        let user_signing = match response.user_signing {
            CrossSigningKeyPublicationWire::Missing => NativeCrossSigningKeyPublication::Missing,
            CrossSigningKeyPublicationWire::Published => {
                NativeCrossSigningKeyPublication::Published
            }
        };
        let private_identity = match response.private_identity {
            CrossSigningPrivateIdentityWire::Missing => NativeCrossSigningPrivateIdentity::Missing,
            CrossSigningPrivateIdentityWire::Partial => NativeCrossSigningPrivateIdentity::Partial,
            CrossSigningPrivateIdentityWire::Complete => {
                NativeCrossSigningPrivateIdentity::Complete
            }
        };
        let own_identity_verification = match response.own_identity_verification {
            OwnIdentityVerificationWire::Missing => NativeOwnIdentityVerification::Missing,
            OwnIdentityVerificationWire::Unverified => NativeOwnIdentityVerification::Unverified,
            OwnIdentityVerificationWire::Verified => NativeOwnIdentityVerification::Verified,
        };
        let bootstrap = match response.bootstrap {
            CrossSigningBootstrapWire::Needed => NativeCrossSigningBootstrap::Needed,
            CrossSigningBootstrapWire::NotNeeded => NativeCrossSigningBootstrap::NotNeeded,
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
}

/// Forward the existing zero-argument Tauri command through Core. No SDK
/// identity, user id, key, secret, client/store, or raw error is accepted by
/// this bridge.
pub(crate) async fn cross_signing_status(
    core: &Core,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: CROSS_SIGNING_STATUS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_cross_signing_status_core_error)?;
    decode_cross_signing_status_response(response)
}

/// Verify Core's complete response envelope and exact legacy truth table. Even
/// though Core constructs the normal response, this stays fail-closed against a
/// future malformed registry/transport implementation and returns no parse text.
fn decode_cross_signing_status_response(
    response: CommandResponseEnvelope,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    if response.command != CROSS_SIGNING_STATUS_COMMAND
        || response.session_generation != READ_ONLY_SESSION_GENERATION
        || response.request_id.is_some()
    {
        return Err(cross_signing_status_response_error());
    }
    let response: CrossSigningStatusWireResponse = serde_json::from_value(response.payload)
        .map_err(|_| cross_signing_status_response_error())?;
    response
        .try_into()
        .map_err(|_| cross_signing_status_response_error())
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
    use std::sync::{Arc, Mutex};

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        CrossSigningStatusFuture, Platform, PlatformCrossSigningStatusError,
        PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig, PlatformStatus,
        PlatformSyncStatus, SecretVault, UnavailableSecretVault,
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

    fn core_returning(
        payload: serde_json::Value,
        forwarded: Arc<Mutex<Vec<CommandEnvelope>>>,
    ) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                CROSS_SIGNING_STATUS_COMMAND,
                move |_state, request| -> CommandFuture {
                    forwarded.lock().expect("test capture lock").push(request);
                    let payload = payload.clone();
                    Box::pin(async move { Ok(payload) })
                },
            )
            .expect("cross-signing status is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    fn core_failing(error: MatrixIpcError) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                CROSS_SIGNING_STATUS_COMMAND,
                move |_state, _request| -> CommandFuture {
                    let error = error.clone();
                    Box::pin(async move { Err(error) })
                },
            )
            .expect("cross-signing status is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    fn response(payload: serde_json::Value) -> CommandResponseEnvelope {
        CommandResponseEnvelope {
            command: CROSS_SIGNING_STATUS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        }
    }

    fn ready_payload(session_generation: u64) -> serde_json::Value {
        serde_json::json!({
            "sessionGeneration": session_generation,
            "readiness": "ready",
            "masterSigning": "published",
            "selfSigning": "published",
            "userSigning": "published",
            "privateIdentity": "complete",
            "ownIdentityVerification": "verified",
            "bootstrap": "not_needed",
        })
    }

    #[tokio::test]
    async fn bridge_forwards_exact_envelope_and_legacy_camel_case_wire_fixture() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = ready_payload(9);
        let status = cross_signing_status(&core_returning(payload.clone(), Arc::clone(&forwarded)))
            .await
            .expect("known Core status remains the legacy desktop DTO");

        assert_eq!(serde_json::to_value(status).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: CROSS_SIGNING_STATUS_COMMAND.to_owned(),
                session_generation: READ_ONLY_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            }]
        );
    }

    #[test]
    fn strict_decoder_rejects_hostile_unknown_malformed_overflow_and_truth_table_violations() {
        let private_text = "@alice:private.example token=secret key=secret";
        let malformed = [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!({ "sessionGeneration": 1, "private": private_text }),
            serde_json::json!({
                "sessionGeneration": 1,
                "readiness": private_text,
                "masterSigning": "published",
                "selfSigning": "published",
                "userSigning": "published",
                "privateIdentity": "complete",
                "ownIdentityVerification": "verified",
                "bootstrap": "not_needed",
            }),
            serde_json::json!({
                "sessionGeneration": MAX_WIRE_COUNTER + 1,
                "readiness": "ready",
                "masterSigning": "published",
                "selfSigning": "published",
                "userSigning": "published",
                "privateIdentity": "complete",
                "ownIdentityVerification": "verified",
                "bootstrap": "not_needed",
            }),
            serde_json::json!({
                "sessionGeneration": 1,
                "readiness": "ready",
                "masterSigning": "published",
                "selfSigning": "missing",
                "userSigning": "published",
                "privateIdentity": "complete",
                "ownIdentityVerification": "verified",
                "bootstrap": "not_needed",
            }),
            serde_json::json!({
                "sessionGeneration": 1,
                "readiness": "recovery_required",
                "masterSigning": "published",
                "selfSigning": "published",
                "userSigning": "published",
                "privateIdentity": "complete",
                "ownIdentityVerification": "verified",
                "bootstrap": "not_needed",
            }),
        ];
        for payload in malformed {
            let error = decode_cross_signing_status_response(response(payload))
                .expect_err("malformed Core status must fail closed");
            assert_eq!(
                error.diagnostic_id,
                "snc-p3-6-cross-signing-status-response-invalid"
            );
            let serialized =
                serde_json::to_string(&error).expect("static desktop error serializes");
            for forbidden in ["alice", "private.example", "token", "secret", "key"] {
                assert!(
                    !serialized.contains(forbidden),
                    "strict decoder must not reflect hostile Core text: {forbidden}"
                );
            }
        }

        for bad_response in [
            CommandResponseEnvelope {
                command: "matrix_cross_signing_setup".to_owned(),
                ..response(ready_payload(1))
            },
            CommandResponseEnvelope {
                session_generation: 1,
                ..response(ready_payload(1))
            },
            CommandResponseEnvelope {
                request_id: Some(private_text.to_owned()),
                ..response(ready_payload(1))
            },
        ] {
            assert_eq!(
                decode_cross_signing_status_response(bad_response)
                    .expect_err("response envelope metadata must be exact")
                    .diagnostic_id,
                "snc-p3-6-cross-signing-status-response-invalid"
            );
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
    async fn bridge_errors_are_static_and_never_reflect_hostile_core_fields() {
        let private_text = "@alice:private.example token=secret password=secret key=secret";
        let unknown = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some(private_text.to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let exact_pair_with_hostile_extras = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some("v-crypto.2-cross-signing-identity-query-failed".to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let unknown_error = cross_signing_status(&core_failing(unknown))
            .await
            .expect_err("unknown Core error must fail closed");
        let legacy_error = cross_signing_status(&core_failing(exact_pair_with_hostile_extras))
            .await
            .expect_err("only the static Core pair is restored");
        assert_eq!(
            unknown_error.diagnostic_id,
            "snc-p3-6-cross-signing-status-core-failed"
        );
        assert_eq!(
            legacy_error.diagnostic_id,
            "v-crypto.2-cross-signing-identity-query-failed"
        );
        for error in [
            unknown_error,
            legacy_error,
            cross_signing_status_core_error(),
            cross_signing_status_response_error(),
        ] {
            let serialized =
                serde_json::to_string(&error).expect("static desktop error serializes");
            for forbidden in [
                "alice",
                "private.example",
                "token",
                "secret",
                "password",
                "key",
            ] {
                assert!(
                    !serialized.contains(forbidden),
                    "bridge error must not reflect private Core data: {forbidden}"
                );
            }
        }
    }
}
