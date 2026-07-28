//! Live V-CRYPTO.3 key-backup and recovery product projections.
//!
//! Recovery keys and passphrases are accepted only by the command layer and
//! passed directly to matrix-sdk. This module never stores or serializes them.

use matrix_sdk::{
    encryption::{
        backups::BackupState,
        recovery::{RecoveryError, RecoveryState},
    },
    ruma::api::{client::backup::get_latest_backup_info, error::ErrorKind},
    Client,
};
use serde::Serialize;
use zeroize::Zeroize;

use crate::matrix::auth::product::MatrixAuthCommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupAvailability {
    Missing,
    Available,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupDeviceState {
    Unavailable,
    Disconnected,
    Connecting,
    Downloading,
    Uploading,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupRecoveryState {
    Unknown,
    NotSetUp,
    Incomplete,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupAction {
    SetupRequired,
    RestoreRequired,
    RepairRequired,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeBackupStatus {
    pub session_generation: u64,
    pub availability: NativeBackupAvailability,
    pub enabled: bool,
    pub version: Option<String>,
    pub key_count: Option<u64>,
    pub device_state: NativeBackupDeviceState,
    pub recovery_state: NativeBackupRecoveryState,
    pub action: NativeBackupAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupOperationOutcome {
    Complete,
    AlreadyConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeBackupOperationResult {
    pub outcome: NativeBackupOperationOutcome,
    pub status: NativeBackupStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerBackupProjection {
    pub version: String,
    pub key_count: u64,
}

pub fn project_status(
    session_generation: u64,
    server: Option<ServerBackupProjection>,
    enabled: bool,
    backup_state: BackupState,
    recovery_state: RecoveryState,
) -> NativeBackupStatus {
    let availability = if server.is_some() {
        NativeBackupAvailability::Available
    } else {
        NativeBackupAvailability::Missing
    };
    let device_state = match backup_state {
        BackupState::Creating | BackupState::Enabling | BackupState::Resuming => {
            NativeBackupDeviceState::Connecting
        }
        BackupState::Downloading => NativeBackupDeviceState::Downloading,
        BackupState::Disabling => NativeBackupDeviceState::Unavailable,
        BackupState::Enabled if enabled => NativeBackupDeviceState::Ready,
        BackupState::Enabled => NativeBackupDeviceState::Uploading,
        BackupState::Unknown if server.is_some() => NativeBackupDeviceState::Disconnected,
        BackupState::Unknown => NativeBackupDeviceState::Unavailable,
    };
    let recovery_state = match recovery_state {
        RecoveryState::Unknown => NativeBackupRecoveryState::Unknown,
        RecoveryState::Disabled => NativeBackupRecoveryState::NotSetUp,
        RecoveryState::Incomplete => NativeBackupRecoveryState::Incomplete,
        RecoveryState::Enabled => NativeBackupRecoveryState::Ready,
    };
    let action = match (availability, enabled, recovery_state) {
        (NativeBackupAvailability::Missing, _, NativeBackupRecoveryState::NotSetUp) => {
            NativeBackupAction::SetupRequired
        }
        (NativeBackupAvailability::Missing, _, NativeBackupRecoveryState::Incomplete) => {
            NativeBackupAction::RepairRequired
        }
        (NativeBackupAvailability::Missing, _, _) => NativeBackupAction::SetupRequired,
        (NativeBackupAvailability::Available, false, NativeBackupRecoveryState::Incomplete) => {
            NativeBackupAction::RepairRequired
        }
        (NativeBackupAvailability::Available, false, _) => NativeBackupAction::RestoreRequired,
        (NativeBackupAvailability::Available, true, NativeBackupRecoveryState::Incomplete) => {
            NativeBackupAction::RepairRequired
        }
        (NativeBackupAvailability::Available, true, _) => NativeBackupAction::None,
    };
    let (version, key_count) = match server {
        Some(server) => (Some(server.version), Some(server.key_count)),
        None => (None, None),
    };

    NativeBackupStatus {
        session_generation,
        availability,
        enabled,
        version,
        key_count,
        device_state,
        recovery_state,
        action,
    }
}

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeBackupStatus, MatrixAuthCommandError> {
    let backups = client.encryption().backups();
    let server = fetch_server_backup(client).await?;
    let enabled = backups.are_enabled().await;
    Ok(project_status(
        session_generation,
        server,
        enabled,
        backups.state(),
        client.encryption().recovery().state(),
    ))
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

async fn fetch_server_backup(
    client: &Client,
) -> Result<Option<ServerBackupProjection>, MatrixAuthCommandError> {
    match client
        .send(get_latest_backup_info::v3::Request::new())
        .await
    {
        Ok(response) => Ok(Some(ServerBackupProjection {
            version: response.version,
            key_count: u64::from(response.count),
        })),
        Err(error) if error.client_api_error_kind() == Some(&ErrorKind::NotFound) => Ok(None),
        Err(_) => Err(backup_error(
            "Unknown",
            "Encryption backup status is unavailable.",
            "v-crypto.3-status-query-failed",
        )),
    }
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
