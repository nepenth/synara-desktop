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

    finish_backup_operation(
        async {
            // Same enrolment as secret-storage bootstrap. This command has no
            // display channel, so the generated key is wiped here. The desktop
            // backup tile enrols through `secret_storage::bootstrap` instead,
            // which returns the key for one-time display.
            let _wiped = crate::app::secret_storage::enable_recovery(client, passphrase)
                .await
                .map_err(|error| match error {
                    RecoveryError::BackupExistsOnServer => "v-crypto.3-setup-existing-backup",
                    _ => "v-crypto.3-setup-failed",
                })?;
            Ok(())
        },
        status(client, session_generation),
        "v-crypto.3-setup-incomplete",
    )
    .await
}

pub async fn restore_operation(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, &'static str> {
    if recovery_secret.trim().is_empty() || recovery_secret.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    finish_backup_operation(
        async {
            client
                .encryption()
                .recovery()
                .recover(recovery_secret)
                .await
                .map_err(|_| "v-crypto.3-restore-rejected")?;
            let _ =
                crate::app::dehydrated_devices::start_with_secret(client, recovery_secret).await;
            Ok(())
        },
        status(client, session_generation),
        "v-crypto.3-restore-incomplete",
    )
    .await
}

pub async fn repair(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, &'static str> {
    if recovery_secret.trim().is_empty() || recovery_secret.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    finish_backup_operation(
        async {
            client
                .encryption()
                .recovery()
                .recover_and_fix_backup(recovery_secret)
                .await
                .map_err(|_| "v-crypto.3-repair-rejected")?;
            let _ =
                crate::app::dehydrated_devices::start_with_secret(client, recovery_secret).await;
            Ok(())
        },
        status(client, session_generation),
        "v-crypto.3-repair-incomplete",
    )
    .await
}

/// SDK mutation and authoritative readback are separate steps. This narrow
/// seam tests partial remote success through the same production completion
/// path; a successful write alone never produces Complete.
async fn finish_backup_operation(
    operation: impl std::future::Future<Output = Result<(), &'static str>>,
    readback: impl std::future::Future<Output = Result<NativeBackupStatus, &'static str>>,
    incomplete_diagnostic_id: &'static str,
) -> Result<NativeBackupOperationResult, &'static str> {
    operation.await?;
    let status = readback.await?;
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
    #[tokio::test]
    async fn successful_backup_mutation_followed_by_incomplete_readback_cannot_complete() {
        use std::cell::Cell;
        let mutated = Cell::new(false);
        let incomplete = project_backup_status(
            8,
            Some(ServerBackupProjection {
                version: "1".into(),
                key_count: 0,
            }),
            true,
            NativeBackupEnginePhase::Enabled,
            NativeBackupRecoveryPhase::Incomplete,
        );
        for diagnostic in [
            "v-crypto.3-setup-incomplete",
            "v-crypto.3-restore-incomplete",
            "v-crypto.3-repair-incomplete",
        ] {
            let result = finish_backup_operation(
                async {
                    mutated.set(true);
                    Ok(())
                },
                async {
                    assert!(mutated.get());
                    Ok(incomplete.clone())
                },
                diagnostic,
            )
            .await;
            assert_eq!(result.unwrap_err(), diagnostic);
            mutated.set(false);
        }
        let read = Cell::new(false);
        let result = finish_backup_operation(
            async { Err("test-sdk-rejected") },
            async {
                read.set(true);
                Ok(incomplete)
            },
            "test-incomplete",
        )
        .await;
        assert_eq!(result.unwrap_err(), "test-sdk-rejected");
        assert!(
            !read.get(),
            "mutation failure must not report a later successful readback"
        );
    }

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
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
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

#[cfg(test)]
mod projection_tests {
    use super::*;
    use crate::app::backup::NativeBackupAction;
    fn project_status(
        generation: u64,
        server: Option<ServerBackupProjection>,
        enabled: bool,
        backup: BackupState,
        recovery: RecoveryState,
    ) -> NativeBackupStatus {
        project_backup_status(
            generation,
            server,
            enabled,
            backup_engine_phase(backup),
            backup_recovery_phase(recovery),
        )
    }
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
