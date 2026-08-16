//! Credential-free V-CRYPTO.3 backup presentation DTOs and projector.
//!
//! Live Client status I/O lives in `live.rs`. Setup/restore/repair stay desktop.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupAvailability {
    Missing,
    Available,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupDeviceState {
    Unavailable,
    Disconnected,
    Connecting,
    Downloading,
    Uploading,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupRecoveryState {
    Unknown,
    NotSetUp,
    Incomplete,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackupAction {
    SetupRequired,
    RestoreRequired,
    RepairRequired,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// SDK-neutral backup engine phase used by the presentation projector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackupEnginePhase {
    Creating,
    Enabling,
    Resuming,
    Downloading,
    Disabling,
    Enabled,
    Unknown,
}

/// SDK-neutral recovery phase used by the presentation projector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackupRecoveryPhase {
    Unknown,
    Disabled,
    Incomplete,
    Enabled,
}

pub fn project_backup_status(
    session_generation: u64,
    server: Option<ServerBackupProjection>,
    enabled: bool,
    backup_phase: NativeBackupEnginePhase,
    recovery_phase: NativeBackupRecoveryPhase,
) -> NativeBackupStatus {
    let availability = if server.is_some() {
        NativeBackupAvailability::Available
    } else {
        NativeBackupAvailability::Missing
    };
    let device_state = match backup_phase {
        NativeBackupEnginePhase::Creating
        | NativeBackupEnginePhase::Enabling
        | NativeBackupEnginePhase::Resuming => NativeBackupDeviceState::Connecting,
        NativeBackupEnginePhase::Downloading => NativeBackupDeviceState::Downloading,
        NativeBackupEnginePhase::Disabling => NativeBackupDeviceState::Unavailable,
        NativeBackupEnginePhase::Enabled if enabled => NativeBackupDeviceState::Ready,
        NativeBackupEnginePhase::Enabled => NativeBackupDeviceState::Uploading,
        NativeBackupEnginePhase::Unknown if server.is_some() => {
            NativeBackupDeviceState::Disconnected
        }
        NativeBackupEnginePhase::Unknown => NativeBackupDeviceState::Unavailable,
    };
    let recovery_state = match recovery_phase {
        NativeBackupRecoveryPhase::Unknown => NativeBackupRecoveryState::Unknown,
        NativeBackupRecoveryPhase::Disabled => NativeBackupRecoveryState::NotSetUp,
        NativeBackupRecoveryPhase::Incomplete => NativeBackupRecoveryState::Incomplete,
        NativeBackupRecoveryPhase::Enabled => NativeBackupRecoveryState::Ready,
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

#[cfg(test)]
mod tests {
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
            project_backup_status(
                1,
                None,
                false,
                NativeBackupEnginePhase::Unknown,
                NativeBackupRecoveryPhase::Disabled,
            )
            .action,
            NativeBackupAction::SetupRequired
        );
        assert_eq!(
            project_backup_status(
                1,
                server(),
                false,
                NativeBackupEnginePhase::Unknown,
                NativeBackupRecoveryPhase::Disabled,
            )
            .action,
            NativeBackupAction::RestoreRequired
        );
        assert_eq!(
            project_backup_status(
                1,
                server(),
                true,
                NativeBackupEnginePhase::Enabled,
                NativeBackupRecoveryPhase::Incomplete,
            )
            .action,
            NativeBackupAction::RepairRequired
        );
        assert_eq!(
            project_backup_status(
                1,
                server(),
                true,
                NativeBackupEnginePhase::Enabled,
                NativeBackupRecoveryPhase::Enabled,
            )
            .action,
            NativeBackupAction::None
        );
    }

    #[test]
    fn status_projection_is_privacy_safe() {
        let status = project_backup_status(
            9,
            server(),
            true,
            NativeBackupEnginePhase::Enabled,
            NativeBackupRecoveryPhase::Enabled,
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
