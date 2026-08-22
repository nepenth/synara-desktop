//! Live V-CRYPTO.3 backup status and restore from the managed Matrix client.
//!
//! Recovery secrets are method arguments only. This module never stores or
//! serializes them.

use matrix_sdk::{
    encryption::{backups::BackupState, recovery::RecoveryState},
    ruma::api::{client::backup::get_latest_backup_info, error::ErrorKind},
    Client,
};

use super::{
    project_backup_status, NativeBackupAvailability, NativeBackupEnginePhase,
    NativeBackupRecoveryPhase, NativeBackupStatus, ServerBackupProjection,
};

/// Privacy-safe restore ack. Status is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixRestoreBackupResult {
    pub status: &'static str,
}

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

async fn fetch_server_backup(
    client: &Client,
) -> Result<Option<ServerBackupProjection>, &'static str> {
    match client
        .send(get_latest_backup_info::v3::Request::new())
        .await
    {
        Ok(response) => Ok(Some(ServerBackupProjection {
            version: response.version,
            key_count: u64::from(response.count),
        })),
        Err(error) if error.client_api_error_kind() == Some(&ErrorKind::NotFound) => Ok(None),
        Err(_) => Err("v-crypto.3-status-query-failed"),
    }
}

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeBackupStatus, &'static str> {
    let backups = client.encryption().backups();
    let server = fetch_server_backup(client).await?;
    let enabled = backups.are_enabled().await;
    Ok(project_backup_status(
        session_generation,
        server,
        enabled,
        backup_engine_phase(backups.state()),
        backup_recovery_phase(client.encryption().recovery().state()),
    ))
}

/// Restore encryption backup with a recovery key or passphrase.
///
/// Empty secret fails closed with a static diagnostic. SDK `recover()`
/// rejection is `v-crypto.3-restore-rejected`. The secret is never copied
/// into the result.
pub async fn restore(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<MatrixRestoreBackupResult, &'static str> {
    if recovery_secret.is_empty() {
        return Err("v-crypto.3-recovery-secret-empty");
    }
    client
        .encryption()
        .recovery()
        .recover(recovery_secret)
        .await
        .map_err(|_| "v-crypto.3-restore-rejected")?;
    let status = status(client, session_generation).await?;
    if !status.enabled || status.availability != NativeBackupAvailability::Available {
        return Err("v-crypto.3-restore-incomplete");
    }
    Ok(MatrixRestoreBackupResult { status: "ok" })
}
