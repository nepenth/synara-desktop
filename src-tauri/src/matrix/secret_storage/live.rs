//! Privacy-safe live Matrix secret-storage product projection.

use serde::Serialize;

use crate::matrix::auth::product::MatrixAuthCommandError;
use matrix_sdk::Client;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSecretStorageSetup {
    #[serde(flatten)]
    pub result: NativeSecretStorageOperationResult,
    /// Shown once in the desktop UI. Never written to Downloads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_key: Option<String>,
}

pub use synara_core::app::secret_storage::{
    operation_result, project_secret_storage_status, NativeMissingSecret, NativeRecoveryPhase,
    NativeSecretStorageAction, NativeSecretStorageOperationResult, NativeSecretStorageOutcome,
    NativeSecretStorageState, NativeSecretStorageStatus,
};

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSecretStorageStatus, MatrixAuthCommandError> {
    let result = synara_core::app::secret_storage::status(client, session_generation)
        .await
        .map_err(map_error)?;
    Ok(result)
}

pub async fn bootstrap(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<DesktopSecretStorageSetup, MatrixAuthCommandError> {
    let result =
        synara_core::app::secret_storage::bootstrap(client, session_generation, passphrase)
            .await
            .map_err(map_error)?;
    Ok(DesktopSecretStorageSetup {
        result: result.result,
        recovery_key: result.recovery_key.map(|key| key.to_string()),
    })
}

pub async fn unlock(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result =
        synara_core::app::secret_storage::unlock(client, session_generation, recovery_secret)
            .await
            .map_err(map_error)?;
    Ok(result)
}

pub async fn reset(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<DesktopSecretStorageSetup, MatrixAuthCommandError> {
    let result = synara_core::app::secret_storage::reset(client, session_generation, passphrase)
        .await
        .map_err(map_error)?;
    Ok(DesktopSecretStorageSetup {
        result: result.result,
        recovery_key: result.recovery_key.map(|key| key.to_string()),
    })
}

fn map_error(diagnostic: &'static str) -> MatrixAuthCommandError {
    let message = match diagnostic {
        "v-crypto.4-bootstrap-cross-signing-required" => {
            "Set up native device verification before enabling secret storage."
        }
        "v-crypto.4-bootstrap-existing-backup" => {
            "Restore the existing encryption backup before setting up secret storage."
        }
        "v-crypto.4-unlock-rejected" => {
            "Secret storage unlock failed. Check your recovery key or passphrase and try again."
        }
        "v-crypto.4-reset-requires-unlock" => {
            "Unlock secret storage before replacing its recovery key."
        }
        "v-crypto.4-operation-incomplete" => "Native secret storage could not be activated.",
        _ => "Native secret storage could not be completed.",
    };
    MatrixAuthCommandError::new("Recovery", message, diagnostic)
}
