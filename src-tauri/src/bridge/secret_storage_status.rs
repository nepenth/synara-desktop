//! Strict desktop bridge for the zero-argument `matrix_secret_storage_status` command.
//!
//! Core owns registry dispatch, payload validation, and exact legacy response
//! serialization. The desktop remains the sole owner of the Matrix SDK client,
//! account-data reads, store, keys, and recovery state. This bridge accepts
//! only the strict public DTO or static Core errors and never reflects text
//! supplied by Core.

use serde::Deserialize;
use synara_core::transport::{
    CommandEnvelope, CommandResponseEnvelope, MatrixIpcError, MatrixIpcErrorCategory,
    MAX_WIRE_COUNTER,
};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::secret_storage::live::{
    NativeMissingSecret, NativeSecretStorageAction, NativeSecretStorageState,
    NativeSecretStorageStatus,
};

const SECRET_STORAGE_STATUS_COMMAND: &str = "matrix_secret_storage_status";
/// The existing status observation takes no renderer payload or generation.
const READ_ONLY_SESSION_GENERATION: u64 = 0;

/// Strict bridge-local decoder for Core's exact legacy camel-case status DTO.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretStorageStatusWireResponse {
    session_generation: u64,
    state: SecretStorageStateWire,
    exists: bool,
    unlocked: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: Vec<MissingSecretWire>,
    action: SecretStorageActionWire,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SecretStorageStateWire {
    Unavailable,
    NotSetUp,
    Locked,
    Ready,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SecretStorageActionWire {
    BootstrapRequired,
    UnlockRequired,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MissingSecretWire {
    CrossSigningMaster,
    CrossSigningSelfSigning,
    CrossSigningUserSigning,
    EncryptionBackup,
}

impl TryFrom<SecretStorageStatusWireResponse> for NativeSecretStorageStatus {
    type Error = ();

    fn try_from(response: SecretStorageStatusWireResponse) -> Result<Self, Self::Error> {
        let state = match response.state {
            SecretStorageStateWire::Unavailable => NativeSecretStorageState::Unavailable,
            SecretStorageStateWire::NotSetUp => NativeSecretStorageState::NotSetUp,
            SecretStorageStateWire::Locked => NativeSecretStorageState::Locked,
            SecretStorageStateWire::Ready => NativeSecretStorageState::Ready,
        };
        let action = match response.action {
            SecretStorageActionWire::BootstrapRequired => {
                NativeSecretStorageAction::BootstrapRequired
            }
            SecretStorageActionWire::UnlockRequired => NativeSecretStorageAction::UnlockRequired,
            SecretStorageActionWire::None => NativeSecretStorageAction::None,
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
}

/// Decode only a canonical, strictly ordered subset of the four legacy public
/// labels. The Core creates this order; requiring it makes malformed/hostile
/// output fail closed rather than become a desktop status object.
fn decode_missing_secrets(values: Vec<MissingSecretWire>) -> Result<Vec<NativeMissingSecret>, ()> {
    let mut previous = None;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let (rank, native) = match value {
            MissingSecretWire::CrossSigningMaster => (0, NativeMissingSecret::CrossSigningMaster),
            MissingSecretWire::CrossSigningSelfSigning => {
                (1, NativeMissingSecret::CrossSigningSelfSigning)
            }
            MissingSecretWire::CrossSigningUserSigning => {
                (2, NativeMissingSecret::CrossSigningUserSigning)
            }
            MissingSecretWire::EncryptionBackup => (3, NativeMissingSecret::EncryptionBackup),
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
        .command(CommandEnvelope {
            command: SECRET_STORAGE_STATUS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_secret_storage_status_core_error)?;
    decode_secret_storage_status_response(response)
}

/// Validate complete Core response metadata plus the exact legacy DTO before
/// constructing the prior desktop response type.
fn decode_secret_storage_status_response(
    response: CommandResponseEnvelope,
) -> Result<NativeSecretStorageStatus, MatrixAuthCommandError> {
    if response.command != SECRET_STORAGE_STATUS_COMMAND
        || response.session_generation != READ_ONLY_SESSION_GENERATION
        || response.request_id.is_some()
    {
        return Err(secret_storage_status_response_error());
    }
    let response: SecretStorageStatusWireResponse = serde_json::from_value(response.payload)
        .map_err(|_| secret_storage_status_response_error())?;
    response
        .try_into()
        .map_err(|_| secret_storage_status_response_error())
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
    use std::sync::{Arc, Mutex};

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        Platform, PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig,
        PlatformSecretStorageStatusError, PlatformStatus, PlatformSyncStatus, SecretVault,
        UnavailableSecretVault,
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

    fn core_returning(
        payload: serde_json::Value,
        forwarded: Arc<Mutex<Vec<CommandEnvelope>>>,
    ) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                SECRET_STORAGE_STATUS_COMMAND,
                move |_state, request| -> CommandFuture {
                    forwarded.lock().expect("test capture lock").push(request);
                    let payload = payload.clone();
                    Box::pin(async move { Ok(payload) })
                },
            )
            .expect("secret-storage status is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    fn core_failing(error: MatrixIpcError) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                SECRET_STORAGE_STATUS_COMMAND,
                move |_state, _request| -> CommandFuture {
                    let error = error.clone();
                    Box::pin(async move { Err(error) })
                },
            )
            .expect("secret-storage status is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    fn response(payload: serde_json::Value) -> CommandResponseEnvelope {
        CommandResponseEnvelope {
            command: SECRET_STORAGE_STATUS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        }
    }

    fn valid_payload(bits: u8, state: &str, unlocked: bool, action: &str) -> serde_json::Value {
        let labels = [
            "cross_signing_master",
            "cross_signing_self_signing",
            "cross_signing_user_signing",
            "encryption_backup",
        ];
        let missing_secrets = labels
            .iter()
            .enumerate()
            .filter_map(|(index, label)| (bits & (1 << index) != 0).then_some(*label))
            .collect::<Vec<_>>();
        serde_json::json!({
            "sessionGeneration": 9,
            "state": state,
            "exists": true,
            "unlocked": unlocked,
            "defaultKeySet": true,
            "passphraseConfigured": true,
            "bootstrapReady": true,
            "missingSecrets": missing_secrets,
            "action": action,
        })
    }

    #[tokio::test]
    async fn bridge_forwards_exact_payload_free_envelope_and_legacy_wire_object() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = valid_payload(0b0101, "locked", false, "unlock_required");
        let status =
            secret_storage_status(&core_returning(payload.clone(), Arc::clone(&forwarded)))
                .await
                .expect("known Core status remains the legacy desktop DTO");

        assert_eq!(serde_json::to_value(status).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: SECRET_STORAGE_STATUS_COMMAND.to_owned(),
                session_generation: READ_ONLY_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            }]
        );
    }

    #[test]
    fn strict_decoder_accepts_every_closed_state_and_missing_secret_case() {
        for (state, unlocked, action) in [
            ("unavailable", false, "unlock_required"),
            ("not_set_up", false, "bootstrap_required"),
            ("locked", false, "unlock_required"),
            ("ready", true, "none"),
        ] {
            for bits in 0_u8..16 {
                let payload = valid_payload(bits, state, unlocked, action);
                let status = decode_secret_storage_status_response(response(payload.clone()))
                    .expect("every fixed legacy state and missing-secret subset is valid");
                assert_eq!(serde_json::to_value(status).unwrap(), payload);
            }
        }
    }

    #[test]
    fn strict_decoder_rejects_hostile_unknown_malformed_overflow_and_invalid_output() {
        let private_text = "https://private.example token=secret recovery_key=secret";
        let malformed = [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!({ "private": private_text }),
            serde_json::json!({
                "sessionGeneration": 9,
                "state": private_text,
                "exists": true,
                "unlocked": false,
                "defaultKeySet": true,
                "passphraseConfigured": true,
                "bootstrapReady": true,
                "missingSecrets": [],
                "action": "unlock_required",
            }),
            serde_json::json!({
                "sessionGeneration": MAX_WIRE_COUNTER + 1,
                "state": "locked",
                "exists": true,
                "unlocked": false,
                "defaultKeySet": true,
                "passphraseConfigured": true,
                "bootstrapReady": true,
                "missingSecrets": [],
                "action": "unlock_required",
            }),
            serde_json::json!({
                "sessionGeneration": 9,
                "state": "ready",
                "exists": true,
                "unlocked": false,
                "defaultKeySet": true,
                "passphraseConfigured": true,
                "bootstrapReady": true,
                "missingSecrets": [],
                "action": "none",
            }),
            serde_json::json!({
                "sessionGeneration": 9,
                "state": "locked",
                "exists": true,
                "unlocked": false,
                "defaultKeySet": true,
                "passphraseConfigured": true,
                "bootstrapReady": true,
                "missingSecrets": ["encryption_backup", "cross_signing_master"],
                "action": "unlock_required",
            }),
            serde_json::json!({
                "sessionGeneration": 9,
                "state": "locked",
                "exists": true,
                "unlocked": false,
                "defaultKeySet": true,
                "passphraseConfigured": true,
                "bootstrapReady": true,
                "missingSecrets": ["cross_signing_master", "cross_signing_master"],
                "action": "unlock_required",
            }),
        ];
        for payload in malformed {
            let error = decode_secret_storage_status_response(response(payload))
                .expect_err("malformed Core status must fail closed");
            assert_eq!(
                error.diagnostic_id,
                "snc-p2-secret-storage-status-response-invalid"
            );
            let serialized = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token=", "recovery_key="] {
                assert!(
                    !serialized.contains(forbidden),
                    "strict decoder must not reflect hostile Core text: {forbidden}"
                );
            }
        }

        for wrong_metadata in [
            CommandResponseEnvelope {
                command: "matrix_secret_storage_unlock".to_owned(),
                ..response(valid_payload(0, "locked", false, "unlock_required"))
            },
            CommandResponseEnvelope {
                session_generation: 1,
                ..response(valid_payload(0, "locked", false, "unlock_required"))
            },
            CommandResponseEnvelope {
                request_id: Some(private_text.to_owned()),
                ..response(valid_payload(0, "locked", false, "unlock_required"))
            },
        ] {
            assert_eq!(
                decode_secret_storage_status_response(wrong_metadata)
                    .expect_err("response metadata must be exact")
                    .diagnostic_id,
                "snc-p2-secret-storage-status-response-invalid"
            );
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
    async fn bridge_errors_are_static_and_never_reflect_hostile_core_fields() {
        let private_text = "https://private.example token=secret recovery_key=secret";
        let unknown = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some(private_text.to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let exact_pair_with_hostile_extras = MatrixIpcError {
            category: MatrixIpcErrorCategory::RecoveryFailure,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some("v-crypto.4-status-secret-check-failed".to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let unknown_error = secret_storage_status(&core_failing(unknown))
            .await
            .expect_err("unknown Core failure must fail closed");
        let legacy_error = secret_storage_status(&core_failing(exact_pair_with_hostile_extras))
            .await
            .expect_err("only exact Core pairs restore old errors");
        assert_eq!(
            unknown_error.diagnostic_id,
            "snc-p2-secret-storage-status-core-failed"
        );
        assert_eq!(
            legacy_error.diagnostic_id,
            "v-crypto.4-status-secret-check-failed"
        );
        for error in [
            unknown_error,
            legacy_error,
            secret_storage_status_core_error(),
            secret_storage_status_response_error(),
        ] {
            let serialized = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token=", "recovery_key="] {
                assert!(
                    !serialized.contains(forbidden),
                    "bridge error must not reflect private Core data: {forbidden}"
                );
            }
        }
    }
}
