//! Live V-CRYPTO.3 backup status from the managed Matrix client.

use matrix_sdk::{
    encryption::{backups::BackupState, recovery::RecoveryState},
    ruma::api::{client::backup::get_latest_backup_info, error::ErrorKind},
    Client,
};

use super::{
    project_backup_status, NativeBackupEnginePhase, NativeBackupRecoveryPhase, NativeBackupStatus,
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
