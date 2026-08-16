//! Live V-CRYPTO.3 key-backup and recovery product projections.
//!
//! Recovery keys and passphrases are accepted only by the command layer and
//! passed directly to matrix-sdk. This module never stores or serializes them.

use matrix_sdk::{
    encryption::{
        backups::BackupState,
        recovery::{RecoveryError, RecoveryState},
    },
    Client,
};
use zeroize::Zeroize;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub use synara_core::app::backup::{
    project_backup_status, NativeBackupAction, NativeBackupAvailability, NativeBackupDeviceState,
    NativeBackupEnginePhase, NativeBackupOperationOutcome, NativeBackupOperationResult,
    NativeBackupRecoveryPhase, NativeBackupRecoveryState, NativeBackupStatus,
    ServerBackupProjection,
};

fn backup_engine_phase(state: BackupState) -> NativeBackupEnginePhase {
    match state {
        BackupState::Creating => NativeBackupEnginePhase::Creating,
        BackupState::Enabling => NativeBackupEnginePhase::Enabling,
        BackupState::Resuming => NativeBackupEnginePhase::Resuming,
        BackupState::Downloading => NativeBackupEnginePhase::Downloading,
        BackupState::Disabling => NativeBackupEnginePhase::Disabling,
        BackupState::Enabled => NativeBackupEnginePhase::Enabled,
        BackupState::Unknown => NativeBackupEnginePhase::Unknown,
    }
}

fn backup_recovery_phase(state: RecoveryState) -> NativeBackupRecoveryPhase {
    match state {
        RecoveryState::Unknown => NativeBackupRecoveryPhase::Unknown,
        RecoveryState::Disabled => NativeBackupRecoveryPhase::Disabled,
        RecoveryState::Incomplete => NativeBackupRecoveryPhase::Incomplete,
        RecoveryState::Enabled => NativeBackupRecoveryPhase::Enabled,
    }
}

pub fn project_status(
    session_generation: u64,
    server: Option<ServerBackupProjection>,
    enabled: bool,
    backup_state: BackupState,
    recovery_state: RecoveryState,
) -> NativeBackupStatus {
    project_backup_status(
        session_generation,
        server,
        enabled,
        backup_engine_phase(backup_state),
        backup_recovery_phase(recovery_state),
    )
}

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeBackupStatus, MatrixAuthCommandError> {
    synara_core::app::backup::status(client, session_generation)
        .await
        .map_err(|diagnostic_id| {
            backup_error(
                "Unknown",
                "Encryption backup status is unavailable.",
                diagnostic_id,
            )
        })
}

pub async fn setup(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let before = status(client, session_generation).await?;
    if before.enabled && before.recovery_state == NativeBackupRecoveryState::Ready {
        return Ok(NativeBackupOperationResult {
            outcome: NativeBackupOperationOutcome::AlreadyConfigured,
            status: before,
        });
    }
    if before.availability == NativeBackupAvailability::Available && !before.enabled {
        return Err(backup_error(
            "InvalidRequest",
            "An existing encryption backup must be restored before setup can continue.",
            "v-crypto.3-setup-existing-backup",
        ));
    }

    let mut generated_recovery_key = client
        .encryption()
        .recovery()
        .enable()
        .with_passphrase(passphrase)
        .wait_for_backups_to_upload()
        .await
        .map_err(map_recovery_setup_error)?;
    generated_recovery_key.zeroize();

    operation_complete(client, session_generation, "v-crypto.3-setup-incomplete").await
}

pub async fn restore(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    client
        .encryption()
        .recovery()
        .recover(recovery_secret)
        .await
        .map_err(|_| {
            backup_error(
                "Forbidden",
                "The recovery key or passphrase was rejected. Check it and try again.",
                "v-crypto.3-restore-rejected",
            )
        })?;
    operation_complete(client, session_generation, "v-crypto.3-restore-incomplete").await
}

pub async fn repair(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    client
        .encryption()
        .recovery()
        .recover_and_fix_backup(recovery_secret)
        .await
        .map_err(|_| {
            backup_error(
                "Forbidden",
                "Encryption backup repair failed. Check your recovery key or passphrase and try again.",
                "v-crypto.3-repair-rejected",
            )
        })?;
    operation_complete(client, session_generation, "v-crypto.3-repair-incomplete").await
}

async fn operation_complete(
    client: &Client,
    session_generation: u64,
    incomplete_diagnostic_id: &'static str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let status = status(client, session_generation).await?;
    if !status.enabled || status.availability != NativeBackupAvailability::Available {
        return Err(backup_error(
            "Unknown",
            "Native encryption backup could not be activated.",
            incomplete_diagnostic_id,
        ));
    }
    Ok(NativeBackupOperationResult {
        outcome: NativeBackupOperationOutcome::Complete,
        status,
    })
}

fn map_recovery_setup_error(error: RecoveryError) -> MatrixAuthCommandError {
    match error {
        RecoveryError::BackupExistsOnServer => backup_error(
            "InvalidRequest",
            "An existing encryption backup must be restored before setup can continue.",
            "v-crypto.3-setup-existing-backup",
        ),
        _ => backup_error(
            "Unknown",
            "Native encryption backup setup could not be completed.",
            "v-crypto.3-setup-failed",
        ),
    }
}

fn backup_error(
    code: &'static str,
    message: &'static str,
    diagnostic_id: &'static str,
) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

#[cfg(test)]
mod tests {
    use matrix_sdk::encryption::{backups::BackupState, recovery::RecoveryState};

    use super::*;

    fn server() -> Option<ServerBackupProjection> {
        Some(ServerBackupProjection {
            version: "7".to_owned(),
            key_count: 42,
        })
    }

    #[test]
    fn projection_covers_setup_restore_repair_and_ready() {
        assert_eq!(
            project_status(
                1,
                None,
                false,
                BackupState::Unknown,
                RecoveryState::Disabled,
            )
            .action,
            NativeBackupAction::SetupRequired
        );
        assert_eq!(
            project_status(
                1,
                server(),
                false,
                BackupState::Unknown,
                RecoveryState::Disabled,
            )
            .action,
            NativeBackupAction::RestoreRequired
        );
        assert_eq!(
            project_status(
                1,
                server(),
                true,
                BackupState::Enabled,
                RecoveryState::Incomplete,
            )
            .action,
            NativeBackupAction::RepairRequired
        );
        assert_eq!(
            project_status(
                1,
                server(),
                true,
                BackupState::Enabled,
                RecoveryState::Enabled,
            )
            .action,
            NativeBackupAction::None
        );
    }

    #[test]
    fn status_projection_is_privacy_safe() {
        let status = project_status(
            9,
            server(),
            true,
            BackupState::Enabled,
            RecoveryState::Enabled,
        );
        let json = serde_json::to_string(&status).unwrap().to_ascii_lowercase();
        assert_eq!(status.version.as_deref(), Some("7"));
        assert_eq!(status.key_count, Some(42));
        for forbidden in [
            "access_token",
            "refresh_token",
            "recovery_key",
            "private_key",
            "ciphertext",
            "passphrase",
            "password",
        ] {
            assert!(!json.contains(forbidden));
        }
    }
}
