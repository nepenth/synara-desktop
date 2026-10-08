//! Typed SharedCore operations and projections for recovery.

use super::*;

/// Explicit one-time key display only; not a status DTO or generic envelope.
#[derive(Clone, uniffi::Record)]
pub struct SecretStorageSetupDto {
    pub status: SecretStorageStatusDto,
    pub recovery_key: Option<String>,
}

/// Privacy-safe secret-storage status from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SecretStorageStatusDto {
    pub session_generation: u64,
    pub state: String,
    pub exists: bool,
    pub unlocked: bool,
    pub default_key_set: bool,
    pub passphrase_configured: bool,
    pub bootstrap_ready: bool,
    pub missing_secrets: Vec<String>,
    pub action: String,
}

/// Privacy-safe leftover backup status. No passphrase or recovery secret.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BackupStatusDto {
    pub session_generation: u64,
    pub availability: String,
    pub enabled: bool,
    pub device_state: String,
    pub recovery_state: String,
    pub action: String,
}

/// Privacy-safe leftover crypto status. No key material.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CryptoStatusDto {
    pub session_generation: u64,
    pub encryption_enabled: bool,
    pub cross_signing_state: String,
}

/// Privacy-safe leftover cross-signing status. No private keys.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CrossSigningStatusDto {
    pub session_generation: u64,
    pub readiness: String,
}

/// Privacy-safe leftover room-key transfer status. No passphrase or path.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomKeyTransferStatusDto {
    pub session_generation: u64,
    pub phase: String,
    pub keys_processed: u32,
}

/// Privacy-safe backup restore ack. Status only; never recovery key or passphrase.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RestoreBackupDto {
    pub status: String,
}

/// Static fail-closed backup restore error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RestoreBackupError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RestoreBackupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RestoreBackupError {}

pub(super) fn restore_backup_failed(code: &str, description: &'static str) -> RestoreBackupError {
    RestoreBackupError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn restore_backup_reject_oversize(size: usize) -> Result<(), RestoreBackupError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(restore_backup_failed(
            RESTORE_BACKUP_FAILED_CODE,
            RESTORE_BACKUP_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

pub(super) fn map_restore_backup_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RestoreBackupError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            restore_backup_failed(code, RESTORE_BACKUP_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-crypto.3-") => {
            restore_backup_failed(code, RESTORE_BACKUP_OWNER_DESCRIPTION)
        }
        _ => restore_backup_failed(
            RESTORE_BACKUP_FAILED_CODE,
            RESTORE_BACKUP_FAILED_DESCRIPTION,
        ),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SecretStorageStatusResultWire {
    pub(super) session_generation: u64,
    pub(super) state: String,
    pub(super) exists: bool,
    pub(super) unlocked: bool,
    pub(super) default_key_set: bool,
    pub(super) passphrase_configured: bool,
    pub(super) bootstrap_ready: bool,
    pub(super) missing_secrets: Vec<String>,
    pub(super) action: String,
}

pub(super) fn closed_secret_storage_state(value: &str) -> Option<&'static str> {
    match value {
        "unavailable" => Some("unavailable"),
        "not_set_up" => Some("not_set_up"),
        "locked" => Some("locked"),
        "ready" => Some("ready"),
        _ => None,
    }
}

pub(super) fn closed_secret_storage_action(value: &str) -> Option<&'static str> {
    match value {
        "bootstrap_required" => Some("bootstrap_required"),
        "unlock_required" => Some("unlock_required"),
        "none" => Some("none"),
        _ => None,
    }
}

pub(super) fn secret_storage_status_dto(
    payload: serde_json::Value,
) -> Result<SecretStorageStatusDto, SessionStatusError> {
    let result: SecretStorageStatusResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let state = closed_secret_storage_state(&result.state).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let action = closed_secret_storage_action(&result.action).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let missing_secrets = result
        .missing_secrets
        .iter()
        .map(|value| {
            closed_missing_secret(value)
                .map(str::to_owned)
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SecretStorageStatusDto {
        session_generation: result.session_generation,
        state: state.to_owned(),
        exists: result.exists,
        unlocked: result.unlocked,
        default_key_set: result.default_key_set,
        passphrase_configured: result.passphrase_configured,
        bootstrap_ready: result.bootstrap_ready,
        missing_secrets,
        action: action.to_owned(),
    })
}

pub(super) fn map_recovery_backup_core_error(_error: MatrixIpcError) -> LeftoverCommandError {
    LeftoverCommandError::Failed {
        code: "recovery-backup-failed".to_owned(),
        description: "Encryption backup recovery could not be completed.".to_owned(),
    }
}

pub(super) fn leftover_backup_status_dto(
    payload: serde_json::Value,
) -> Result<BackupStatusDto, LeftoverCommandError> {
    Ok(BackupStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        availability: payload
            .get("availability")
            .and_then(|value| value.as_str())
            .unwrap_or("missing")
            .to_owned(),
        enabled: payload
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        device_state: payload
            .get("deviceState")
            .and_then(|value| value.as_str())
            .unwrap_or("unavailable")
            .to_owned(),
        recovery_state: payload
            .get("recoveryState")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_owned(),
        action: payload
            .get("action")
            .and_then(|value| value.as_str())
            .unwrap_or("none")
            .to_owned(),
    })
}

pub(super) fn leftover_crypto_status_dto(
    payload: serde_json::Value,
) -> Result<CryptoStatusDto, LeftoverCommandError> {
    Ok(CryptoStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        encryption_enabled: payload
            .get("encryptionEnabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        cross_signing_state: payload
            .get("crossSigningState")
            .and_then(|value| value.as_str())
            .unwrap_or("unavailable")
            .to_owned(),
    })
}

pub(super) fn leftover_cross_signing_status_dto(
    payload: serde_json::Value,
) -> Result<CrossSigningStatusDto, LeftoverCommandError> {
    Ok(CrossSigningStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        readiness: payload
            .get("readiness")
            .and_then(|value| value.as_str())
            .unwrap_or("unavailable")
            .to_owned(),
    })
}

pub(super) fn leftover_room_key_transfer_status_dto(
    payload: serde_json::Value,
) -> Result<RoomKeyTransferStatusDto, LeftoverCommandError> {
    Ok(RoomKeyTransferStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        phase: payload
            .get("phase")
            .and_then(|value| value.as_str())
            .unwrap_or("idle")
            .to_owned(),
        keys_processed: payload
            .get("keysProcessed")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32,
    })
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// Restore encryption backup. Recovery secret is a dedicated FFI argument,
    /// never a Core JSON field. Leftover `recover` remains fail-closed.
    pub async fn restore_backup(
        &self,
        recovery_secret: String,
    ) -> Result<RestoreBackupDto, RestoreBackupError> {
        let recovery_secret = Zeroizing::new(recovery_secret);
        restore_backup_reject_oversize(recovery_secret.len())?;
        let result = self
            .core
            .restore_backup(recovery_secret.as_str())
            .await
            .map_err(|error| {
                map_restore_backup_core_error(RESTORE_BACKUP_NO_SESSION_CODE, error)
            })?;
        Ok(RestoreBackupDto {
            status: result.status.to_owned(),
        })
    }

    pub async fn secret_storage_status(
        &self,
    ) -> Result<SecretStorageStatusDto, SessionStatusError> {
        let payload = self
            .session_status_command(SECRET_STORAGE_STATUS_COMMAND)
            .await?;
        secret_storage_status_dto(payload)
    }

    /// Typed recovery arguments bypass generic command envelopes. Returned keys
    /// are only the explicit once-displayed bootstrap/reset contract.
    pub async fn secret_storage_bootstrap(
        &self,
        passphrase: String,
    ) -> Result<SecretStorageSetupDto, SessionStatusError> {
        let secret = Zeroizing::new(passphrase);
        let result = self
            .core
            .secret_storage_bootstrap(secret.as_str())
            .await
            .map_err(map_session_status_core_error)?;
        let status = secret_storage_status_dto(
            serde_json::to_value(result.result.status).map_err(|_| {
                session_status_failed(
                    "recovery-projection-failed",
                    "Recovery status is unavailable.",
                )
            })?,
        )?;
        Ok(SecretStorageSetupDto {
            status,
            recovery_key: result.recovery_key.map(|key| key.to_string()),
        })
    }

    pub async fn secret_storage_unlock(
        &self,
        recovery_secret: String,
    ) -> Result<SecretStorageStatusDto, SessionStatusError> {
        let secret = Zeroizing::new(recovery_secret);
        let result = self
            .core
            .secret_storage_unlock(secret.as_str())
            .await
            .map_err(map_session_status_core_error)?;
        secret_storage_status_dto(serde_json::to_value(result.status).map_err(|_| {
            session_status_failed(
                "recovery-projection-failed",
                "Recovery status is unavailable.",
            )
        })?)
    }

    pub async fn secret_storage_reset(
        &self,
        passphrase: String,
    ) -> Result<SecretStorageSetupDto, SessionStatusError> {
        let secret = Zeroizing::new(passphrase);
        let result = self
            .core
            .secret_storage_reset(secret.as_str())
            .await
            .map_err(map_session_status_core_error)?;
        let status = secret_storage_status_dto(
            serde_json::to_value(result.result.status).map_err(|_| {
                session_status_failed(
                    "recovery-projection-failed",
                    "Recovery status is unavailable.",
                )
            })?,
        )?;
        Ok(SecretStorageSetupDto {
            status,
            recovery_key: result.recovery_key.map(|key| key.to_string()),
        })
    }

    pub async fn backup_setup(
        &self,
        passphrase: String,
    ) -> Result<BackupStatusDto, LeftoverCommandError> {
        let secret = Zeroizing::new(passphrase);
        let result = self
            .core
            .backup_setup(secret.as_str())
            .await
            .map_err(map_recovery_backup_core_error)?;
        leftover_backup_status_dto(serde_json::to_value(result.status).map_err(|_| {
            map_recovery_backup_core_error(MatrixIpcError::new(
                MatrixIpcErrorCategory::SdkInvariant,
            ))
        })?)
    }

    pub async fn backup_repair(
        &self,
        recovery_secret: String,
    ) -> Result<BackupStatusDto, LeftoverCommandError> {
        let secret = Zeroizing::new(recovery_secret);
        let result = self
            .core
            .backup_repair(secret.as_str())
            .await
            .map_err(map_recovery_backup_core_error)?;
        leftover_backup_status_dto(serde_json::to_value(result.status).map_err(|_| {
            map_recovery_backup_core_error(MatrixIpcError::new(
                MatrixIpcErrorCategory::SdkInvariant,
            ))
        })?)
    }

    pub async fn backup_status(&self) -> Result<BackupStatusDto, LeftoverCommandError> {
        let payload = self.leftover_status_command(BACKUP_STATUS_COMMAND).await?;
        leftover_backup_status_dto(payload)
    }

    pub async fn crypto_status(&self) -> Result<CryptoStatusDto, LeftoverCommandError> {
        let payload = self.leftover_status_command(CRYPTO_STATUS_COMMAND).await?;
        leftover_crypto_status_dto(payload)
    }

    pub async fn cross_signing_status(
        &self,
    ) -> Result<CrossSigningStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(CROSS_SIGNING_STATUS_COMMAND)
            .await?;
        leftover_cross_signing_status_dto(payload)
    }

    pub async fn room_key_transfer_status(
        &self,
    ) -> Result<RoomKeyTransferStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(ROOM_KEY_TRANSFER_STATUS_COMMAND)
            .await?;
        leftover_room_key_transfer_status_dto(payload)
    }
}
