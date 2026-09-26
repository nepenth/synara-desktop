//! Privacy-safe live Matrix secret-storage product projection.

use serde::Serialize;

use matrix_sdk::{
    encryption::recovery::{RecoveryError, RecoveryState},
    ruma::events::{
        secret::request::SecretName,
        secret_storage::{key::SecretStorageKeyEventContent, secret::SecretEventContent},
        EventContentFromType, GlobalAccountDataEventType,
    },
    Client,
};
use zeroize::Zeroize;

use crate::matrix::auth::product::MatrixAuthCommandError;

#[derive(Debug, Serialize)]
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
    let secret_storage = client.encryption().secret_storage();
    let default_key = secret_storage.fetch_default_key_id().await.map_err(|_| {
        secret_storage_error(
            "Native secret storage status is unavailable.",
            "v-crypto.4-status-default-key-failed",
        )
    })?;
    let default_key = default_key.and_then(|raw| raw.deserialize().ok());
    let default_key_set = default_key.is_some();
    let default_key_id = default_key.as_ref().map(|content| content.key_id.as_str());
    let (exists, passphrase_configured) = match default_key.as_ref() {
        Some(default_key) => {
            let event_type =
                GlobalAccountDataEventType::SecretStorageKey(default_key.key_id.to_owned());
            let key = client
                .account()
                .fetch_account_data(event_type.to_owned())
                .await
                .map_err(|_| {
                    secret_storage_error(
                        "Native secret storage status is unavailable.",
                        "v-crypto.4-status-key-info-failed",
                    )
                })?;
            let key = key.and_then(|raw| {
                let event_type = event_type.to_string();
                serde_json::value::to_raw_value(&raw)
                    .ok()
                    .and_then(|value| {
                        SecretStorageKeyEventContent::from_parts(&event_type, &value).ok()
                    })
            });
            (
                key.is_some(),
                key.as_ref()
                    .is_some_and(|content| content.passphrase.is_some()),
            )
        }
        None => (false, false),
    };

    let missing_secrets = missing_secrets(client, default_key_id).await?;
    let bootstrap_ready = client
        .encryption()
        .cross_signing_status()
        .await
        .is_some_and(|status| status.is_complete());
    Ok(project_status(
        session_generation,
        client.encryption().recovery().state(),
        exists,
        default_key_set,
        passphrase_configured,
        bootstrap_ready,
        missing_secrets,
    ))
}

pub async fn bootstrap(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<DesktopSecretStorageSetup, MatrixAuthCommandError> {
    let before = status(client, session_generation).await?;
    if before.exists {
        return Ok(DesktopSecretStorageSetup {
            result: operation_result(NativeSecretStorageOutcome::AlreadyConfigured, false, before),
            recovery_key: None,
        });
    }
    if !before.bootstrap_ready {
        return Err(secret_storage_error(
            "Set up native device verification before enabling secret storage.",
            "v-crypto.4-bootstrap-cross-signing-required",
        ));
    }

    let mut recovery_key = client
        .encryption()
        .recovery()
        .enable()
        .with_passphrase(passphrase)
        .wait_for_backups_to_upload()
        .await
        .map_err(map_bootstrap_error)?;
    let displayed_key = recovery_key.clone();
    let _ = synara_core::app::dehydrated_devices::start_with_secret(client, &recovery_key).await;
    recovery_key.zeroize();

    Ok(DesktopSecretStorageSetup {
        result: operation_result(
            NativeSecretStorageOutcome::Complete,
            false,
            status(client, session_generation).await?,
        ),
        recovery_key: Some(displayed_key),
    })
}

pub async fn unlock(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    client
        .encryption()
        .recovery()
        .recover(recovery_secret)
        .await
        .map_err(|_| {
            secret_storage_error(
                "Secret storage unlock failed. Check your recovery key or passphrase and try again.",
                "v-crypto.4-unlock-rejected",
            )
        })?;
    let _ = synara_core::app::dehydrated_devices::start_with_secret(client, recovery_secret).await;
    Ok(operation_result(
        NativeSecretStorageOutcome::Complete,
        false,
        status(client, session_generation).await?,
    ))
}

pub async fn reset(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<DesktopSecretStorageSetup, MatrixAuthCommandError> {
    let before = status(client, session_generation).await?;
    if !before.unlocked {
        return Err(secret_storage_error(
            "Unlock secret storage before replacing its recovery key.",
            "v-crypto.4-reset-requires-unlock",
        ));
    }

    let mut recovery_key = client
        .encryption()
        .recovery()
        .reset_key()
        .with_passphrase(passphrase)
        .await
        .map_err(|_| {
            secret_storage_error(
                "Native secret storage reset could not be completed.",
                "v-crypto.4-reset-failed",
            )
        })?;
    let displayed_key = recovery_key.clone();
    let _ = synara_core::app::dehydrated_devices::start_with_secret(client, &recovery_key).await;
    recovery_key.zeroize();

    Ok(DesktopSecretStorageSetup {
        result: operation_result(
            NativeSecretStorageOutcome::Complete,
            false,
            status(client, session_generation).await?,
        ),
        recovery_key: Some(displayed_key),
    })
}

fn recovery_phase(state: RecoveryState) -> NativeRecoveryPhase {
    match state {
        RecoveryState::Unknown => NativeRecoveryPhase::Unknown,
        RecoveryState::Disabled => NativeRecoveryPhase::Disabled,
        RecoveryState::Incomplete => NativeRecoveryPhase::Incomplete,
        RecoveryState::Enabled => NativeRecoveryPhase::Enabled,
    }
}

async fn missing_secrets(
    client: &Client,
    default_key_id: Option<&str>,
) -> Result<Vec<NativeMissingSecret>, MatrixAuthCommandError> {
    let known = [
        (
            SecretName::CrossSigningMasterKey,
            NativeMissingSecret::CrossSigningMaster,
        ),
        (
            SecretName::CrossSigningSelfSigningKey,
            NativeMissingSecret::CrossSigningSelfSigning,
        ),
        (
            SecretName::CrossSigningUserSigningKey,
            NativeMissingSecret::CrossSigningUserSigning,
        ),
        (
            SecretName::RecoveryKey,
            NativeMissingSecret::EncryptionBackup,
        ),
    ];
    let mut missing = Vec::new();
    for (name, projection) in known {
        let event_type = GlobalAccountDataEventType::from(name);
        let content = client
            .account()
            .fetch_account_data(event_type)
            .await
            .map_err(|_| {
                secret_storage_error(
                    "Native secret storage status is unavailable.",
                    "v-crypto.4-status-secret-check-failed",
                )
            })?;
        let present = content
            .and_then(|raw| raw.deserialize_as_unchecked::<SecretEventContent>().ok())
            .is_some_and(|content| {
                default_key_id.is_some_and(|key_id| content.encrypted.contains_key(key_id))
            });
        if !present {
            missing.push(projection);
        }
    }
    Ok(missing)
}

fn project_status(
    session_generation: u64,
    recovery_state: RecoveryState,
    exists: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: Vec<NativeMissingSecret>,
) -> NativeSecretStorageStatus {
    project_secret_storage_status(
        session_generation,
        recovery_phase(recovery_state),
        exists,
        default_key_set,
        passphrase_configured,
        bootstrap_ready,
        missing_secrets,
    )
}

fn map_bootstrap_error(error: RecoveryError) -> MatrixAuthCommandError {
    match error {
        RecoveryError::BackupExistsOnServer => secret_storage_error(
            "Restore the existing encryption backup before setting up secret storage.",
            "v-crypto.4-bootstrap-existing-backup",
        ),
        _ => secret_storage_error(
            "Native secret storage setup could not be completed.",
            "v-crypto.4-bootstrap-failed",
        ),
    }
}

fn secret_storage_error(
    message: &'static str,
    diagnostic_id: &'static str,
) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new("Recovery", message, diagnostic_id)
}
