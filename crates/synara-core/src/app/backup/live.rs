//! Live V-CRYPTO.3 backup status and restore from the managed Matrix client.
//!
//! Recovery secrets are method arguments only. This module never stores or
//! serializes them.

use matrix_sdk::{
    encryption::{
        backups::BackupState,
        recovery::{RecoveryError, RecoveryState},
    },
    ruma::api::{client::backup::get_latest_backup_info, error::ErrorKind},
    Client,
};

use super::{
    project_backup_status, NativeBackupAvailability, NativeBackupEnginePhase,
    NativeBackupOperationOutcome, NativeBackupOperationResult, NativeBackupRecoveryPhase,
    NativeBackupRecoveryState, NativeBackupStatus, ServerBackupProjection,
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
    restore_operation(client, session_generation, recovery_secret).await?;
    Ok(MatrixRestoreBackupResult { status: "ok" })
}

pub async fn setup(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<NativeBackupOperationResult, &'static str> {
    if passphrase.trim().is_empty() || passphrase.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    let before = status(client, session_generation).await?;
    if backup_is_complete(&before) {
        return Ok(NativeBackupOperationResult {
            outcome: NativeBackupOperationOutcome::AlreadyConfigured,
            status: before,
        });
    }
    if before.availability == NativeBackupAvailability::Available && !before.enabled {
        return Err("v-crypto.3-setup-existing-backup");
    }

    let generated_recovery_key = zeroize::Zeroizing::new(
        client
            .encryption()
            .recovery()
            .enable()
            .with_passphrase(passphrase)
            .wait_for_backups_to_upload()
            .await
            .map_err(|error| match error {
                RecoveryError::BackupExistsOnServer => "v-crypto.3-setup-existing-backup",
                _ => "v-crypto.3-setup-failed",
            })?,
    );
    let _ =
        crate::app::dehydrated_devices::start_with_secret(client, &generated_recovery_key).await;

    operation_complete(client, session_generation, "v-crypto.3-setup-incomplete").await
}

pub async fn restore_operation(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, &'static str> {
    if recovery_secret.trim().is_empty() || recovery_secret.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    client
        .encryption()
        .recovery()
        .recover(recovery_secret)
        .await
        .map_err(|_| "v-crypto.3-restore-rejected")?;
    let _ = crate::app::dehydrated_devices::start_with_secret(client, recovery_secret).await;
    operation_complete(client, session_generation, "v-crypto.3-restore-incomplete").await
}

pub async fn repair(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, &'static str> {
    if recovery_secret.trim().is_empty() || recovery_secret.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    client
        .encryption()
        .recovery()
        .recover_and_fix_backup(recovery_secret)
        .await
        .map_err(|_| "v-crypto.3-repair-rejected")?;
    let _ = crate::app::dehydrated_devices::start_with_secret(client, recovery_secret).await;
    operation_complete(client, session_generation, "v-crypto.3-repair-incomplete").await
}

async fn operation_complete(
    client: &Client,
    session_generation: u64,
    incomplete_diagnostic_id: &'static str,
) -> Result<NativeBackupOperationResult, &'static str> {
    let status = status(client, session_generation).await?;
    if !backup_is_complete(&status) {
        return Err(incomplete_diagnostic_id);
    }
    Ok(NativeBackupOperationResult {
        outcome: NativeBackupOperationOutcome::Complete,
        status,
    })
}

fn backup_is_complete(status: &NativeBackupStatus) -> bool {
    status.enabled
        && status.availability == NativeBackupAvailability::Available
        && status.recovery_state == NativeBackupRecoveryState::Ready
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_completion_rejects_partial_native_state() {
        let ready = project_backup_status(
            8,
            Some(ServerBackupProjection {
                version: "1".into(),
                key_count: 3,
            }),
            true,
            NativeBackupEnginePhase::Enabled,
            NativeBackupRecoveryPhase::Enabled,
        );
        assert!(backup_is_complete(&ready));
        let mut disabled = ready.clone();
        disabled.enabled = false;
        assert!(!backup_is_complete(&disabled));
        let mut missing = ready.clone();
        missing.availability = NativeBackupAvailability::Missing;
        assert!(!backup_is_complete(&missing));
        let mut incomplete = ready;
        incomplete.recovery_state = NativeBackupRecoveryState::Incomplete;
        assert!(!backup_is_complete(&incomplete));
    }
    #[tokio::test]
    async fn backup_sdk_query_failure_is_static_and_never_complete() {
        use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };
        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .request_config(RequestConfig::new().disable_retry())
            .build()
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/_matrix/client/.*/room_keys/version$"))
            .respond_with(ResponseTemplate::new(503).set_body_json(
                serde_json::json!({"errcode":"M_UNKNOWN","error":"private SDK details"}),
            ))
            .mount(server.server())
            .await;
        assert_eq!(
            status(&client, 8).await.unwrap_err(),
            "v-crypto.3-status-query-failed"
        );
        assert_eq!(
            setup(&client, 8, "test-passphrase").await.unwrap_err(),
            "v-crypto.3-status-query-failed"
        );
    }
}
