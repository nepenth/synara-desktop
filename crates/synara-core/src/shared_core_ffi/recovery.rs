//! Typed SharedCore operations and projections for recovery.

use super::*;
use crate::app::backup::NativeBackupStatus;
use crate::app::room_keys::NativeRoomKeyTransferStatus;
use crate::core_api::MatrixCrossSigningStatusResponse;
use crate::core_api::MatrixSecretStorageStatusResponse;
use crate::MatrixCryptoStatus;

use super::wire_enum::{wire_enum, wire_enum_from};
use crate::app::backup::{
    NativeBackupAction, NativeBackupAvailability, NativeBackupDeviceState,
    NativeBackupRecoveryState,
};
use crate::app::room_keys::NativeRoomKeyTransferPhase;
use crate::core_api::{
    MatrixCrossSigningReadinessResponse, MatrixMissingSecretResponse,
    MatrixSecretStorageActionResponse, MatrixSecretStorageStateResponse,
};
use crate::MatrixCrossSigningState;

wire_enum! {
    pub enum SecretStorageStateDto {
        Unavailable => "unavailable",
        NotSetUp => "not_set_up",
        Locked => "locked",
        Ready => "ready",
    }
}
wire_enum_from!(MatrixSecretStorageStateResponse => SecretStorageStateDto {
    Unavailable, NotSetUp, Locked, Ready
});

wire_enum! {
    pub enum SecretStorageActionDto {
        BootstrapRequired => "bootstrap_required",
        UnlockRequired => "unlock_required",
        NoAction => "none",
    }
}

impl From<MatrixSecretStorageActionResponse> for SecretStorageActionDto {
    fn from(action: MatrixSecretStorageActionResponse) -> Self {
        match action {
            MatrixSecretStorageActionResponse::BootstrapRequired => Self::BootstrapRequired,
            MatrixSecretStorageActionResponse::UnlockRequired => Self::UnlockRequired,
            MatrixSecretStorageActionResponse::None => Self::NoAction,
        }
    }
}

wire_enum! {
    pub enum MissingSecretDto {
        CrossSigningMaster => "cross_signing_master",
        CrossSigningSelfSigning => "cross_signing_self_signing",
        CrossSigningUserSigning => "cross_signing_user_signing",
        EncryptionBackup => "encryption_backup",
    }
}
wire_enum_from!(MatrixMissingSecretResponse => MissingSecretDto {
    CrossSigningMaster, CrossSigningSelfSigning, CrossSigningUserSigning, EncryptionBackup
});

wire_enum! {
    pub enum BackupAvailabilityDto {
        Missing => "missing",
        Available => "available",
    }
}
wire_enum_from!(NativeBackupAvailability => BackupAvailabilityDto { Missing, Available });

wire_enum! {
    pub enum BackupDeviceStateDto {
        Unavailable => "unavailable",
        Disconnected => "disconnected",
        Connecting => "connecting",
        Downloading => "downloading",
        Uploading => "uploading",
        Ready => "ready",
    }
}
wire_enum_from!(NativeBackupDeviceState => BackupDeviceStateDto {
    Unavailable, Disconnected, Connecting, Downloading, Uploading, Ready
});

wire_enum! {
    pub enum BackupRecoveryStateDto {
        Unknown => "unknown",
        NotSetUp => "not_set_up",
        Incomplete => "incomplete",
        Ready => "ready",
    }
}
wire_enum_from!(NativeBackupRecoveryState => BackupRecoveryStateDto {
    Unknown, NotSetUp, Incomplete, Ready
});

wire_enum! {
    pub enum BackupActionDto {
        SetupRequired => "setup_required",
        RestoreRequired => "restore_required",
        RepairRequired => "repair_required",
        NoAction => "none",
    }
}

impl From<NativeBackupAction> for BackupActionDto {
    fn from(action: NativeBackupAction) -> Self {
        match action {
            NativeBackupAction::SetupRequired => Self::SetupRequired,
            NativeBackupAction::RestoreRequired => Self::RestoreRequired,
            NativeBackupAction::RepairRequired => Self::RepairRequired,
            NativeBackupAction::None => Self::NoAction,
        }
    }
}

wire_enum! {
    pub enum CrossSigningStateDto {
        Unavailable => "unavailable",
        NotSetUp => "not_set_up",
        Partial => "partial",
        Ready => "ready",
    }
}
wire_enum_from!(MatrixCrossSigningState => CrossSigningStateDto {
    Unavailable, NotSetUp, Partial, Ready
});

wire_enum! {
    pub enum CrossSigningReadinessDto {
        Unavailable => "unavailable",
        SetupRequired => "setup_required",
        RecoveryRequired => "recovery_required",
        VerificationRequired => "verification_required",
        Ready => "ready",
    }
}
wire_enum_from!(MatrixCrossSigningReadinessResponse => CrossSigningReadinessDto {
    Unavailable, SetupRequired, RecoveryRequired, VerificationRequired, Ready
});

wire_enum! {
    pub enum RoomKeyTransferPhaseDto {
        Idle => "idle",
        Preparing => "preparing",
        InFlight => "in_flight",
        Succeeded => "succeeded",
        Failed => "failed",
        Cancelled => "cancelled",
    }
}
wire_enum_from!(NativeRoomKeyTransferPhase => RoomKeyTransferPhaseDto {
    Idle, Preparing, InFlight, Succeeded, Failed, Cancelled
});

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
    pub state: SecretStorageStateDto,
    pub exists: bool,
    pub unlocked: bool,
    pub default_key_set: bool,
    pub passphrase_configured: bool,
    pub bootstrap_ready: bool,
    pub missing_secrets: Vec<MissingSecretDto>,
    pub action: SecretStorageActionDto,
}

/// Privacy-safe leftover backup status. No passphrase or recovery secret.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BackupStatusDto {
    pub session_generation: u64,
    pub availability: BackupAvailabilityDto,
    pub enabled: bool,
    pub device_state: BackupDeviceStateDto,
    pub recovery_state: BackupRecoveryStateDto,
    pub action: BackupActionDto,
}

/// Privacy-safe leftover crypto status. No key material.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CryptoStatusDto {
    pub session_generation: u64,
    pub encryption_enabled: bool,
    pub cross_signing_state: CrossSigningStateDto,
}

/// Privacy-safe leftover cross-signing status. No private keys.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CrossSigningStatusDto {
    pub session_generation: u64,
    pub readiness: CrossSigningReadinessDto,
}

/// Privacy-safe leftover room-key transfer status. No passphrase or path.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomKeyTransferStatusDto {
    pub session_generation: u64,
    pub phase: RoomKeyTransferPhaseDto,
    pub keys_processed: u32,
}

/// Privacy-safe backup restore ack. Status only; never recovery key or passphrase.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RestoreBackupDto {
    pub status: WriteAckDto,
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

pub(super) fn secret_storage_status_dto(
    payload: MatrixSecretStorageStatusResponse,
) -> Result<SecretStorageStatusDto, SessionStatusError> {
    Ok(SecretStorageStatusDto {
        session_generation: payload.session_generation,
        state: payload.state.into(),
        exists: payload.exists,
        unlocked: payload.unlocked,
        default_key_set: payload.default_key_set,
        passphrase_configured: payload.passphrase_configured,
        bootstrap_ready: payload.bootstrap_ready,
        missing_secrets: payload
            .missing_secrets
            .into_iter()
            .map(Into::into)
            .collect(),
        action: payload.action.into(),
    })
}

pub(super) fn map_recovery_backup_core_error(_error: MatrixIpcError) -> LeftoverCommandError {
    LeftoverCommandError::Failed {
        code: "recovery-backup-failed".to_owned(),
        description: "Encryption backup recovery could not be completed.".to_owned(),
    }
}

pub(super) fn leftover_backup_status_dto(
    payload: NativeBackupStatus,
) -> Result<BackupStatusDto, LeftoverCommandError> {
    Ok(BackupStatusDto {
        session_generation: payload.session_generation,
        availability: payload.availability.into(),
        enabled: payload.enabled,
        device_state: payload.device_state.into(),
        recovery_state: payload.recovery_state.into(),
        action: payload.action.into(),
    })
}

pub(super) fn leftover_crypto_status_dto(
    payload: MatrixCryptoStatus,
) -> Result<CryptoStatusDto, LeftoverCommandError> {
    Ok(CryptoStatusDto {
        session_generation: payload.session_generation,
        encryption_enabled: payload.encryption_enabled,
        cross_signing_state: payload.cross_signing_state.into(),
    })
}

pub(super) fn leftover_cross_signing_status_dto(
    payload: MatrixCrossSigningStatusResponse,
) -> Result<CrossSigningStatusDto, LeftoverCommandError> {
    Ok(CrossSigningStatusDto {
        session_generation: payload.session_generation,
        readiness: payload.readiness.into(),
    })
}

pub(super) fn leftover_room_key_transfer_status_dto(
    payload: NativeRoomKeyTransferStatus,
) -> Result<RoomKeyTransferStatusDto, LeftoverCommandError> {
    Ok(RoomKeyTransferStatusDto {
        session_generation: payload.session_generation,
        phase: payload.phase.into(),
        keys_processed: payload.keys_processed,
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
            status: result.status.into(),
        })
    }

    pub async fn secret_storage_status(
        &self,
    ) -> Result<SecretStorageStatusDto, SessionStatusError> {
        let payload = self
            .session_status_command(self.core.secret_storage_status())
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
        let status = secret_storage_status_dto(MatrixSecretStorageStatusResponse::from(
            result.result.status,
        ))?;
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
        secret_storage_status_dto(MatrixSecretStorageStatusResponse::from(result.status))
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
        let status = secret_storage_status_dto(MatrixSecretStorageStatusResponse::from(
            result.result.status,
        ))?;
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
        leftover_backup_status_dto(result.status)
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
        leftover_backup_status_dto(result.status)
    }

    pub async fn backup_status(&self) -> Result<BackupStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(self.core.backup_status())
            .await?;
        leftover_backup_status_dto(payload)
    }

    pub async fn crypto_status(&self) -> Result<CryptoStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(self.core.crypto_status())
            .await?;
        leftover_crypto_status_dto(payload)
    }

    pub async fn cross_signing_status(
        &self,
    ) -> Result<CrossSigningStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(self.core.cross_signing_status())
            .await?;
        leftover_cross_signing_status_dto(payload)
    }

    pub async fn room_key_transfer_status(
        &self,
    ) -> Result<RoomKeyTransferStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(self.core.room_key_transfer_status())
            .await?;
        leftover_room_key_transfer_status_dto(payload)
    }
}
