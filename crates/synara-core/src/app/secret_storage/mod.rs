//! Credential-free V-CRYPTO.4 secret-storage presentation DTOs.
//!
//! Live Client recovery I/O and host recovery-document writes stay in the desktop shell.

use serde::Serialize;

pub const RECOVERY_DOCUMENT_NAME: &str = "synara-recovery-key.txt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeSecretStorageState {
    Unavailable,
    NotSetUp,
    Locked,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeSecretStorageAction {
    BootstrapRequired,
    UnlockRequired,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeMissingSecret {
    CrossSigningMaster,
    CrossSigningSelfSigning,
    CrossSigningUserSigning,
    EncryptionBackup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSecretStorageStatus {
    pub session_generation: u64,
    pub state: NativeSecretStorageState,
    pub exists: bool,
    pub unlocked: bool,
    pub default_key_set: bool,
    pub passphrase_configured: bool,
    pub bootstrap_ready: bool,
    pub missing_secrets: Vec<NativeMissingSecret>,
    pub action: NativeSecretStorageAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeSecretStorageOutcome {
    Complete,
    AlreadyConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSecretStorageOperationResult {
    pub outcome: NativeSecretStorageOutcome,
    pub recovery_document_saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_document_name: Option<&'static str>,
    pub status: NativeSecretStorageStatus,
}

/// SDK-neutral recovery phase used by the presentation projector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRecoveryPhase {
    Unknown,
    Disabled,
    Incomplete,
    Enabled,
}

pub fn project_secret_storage_status(
    session_generation: u64,
    recovery_phase: NativeRecoveryPhase,
    exists: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: Vec<NativeMissingSecret>,
) -> NativeSecretStorageStatus {
    let (state, unlocked, action) = match recovery_phase {
        NativeRecoveryPhase::Unknown => (
            NativeSecretStorageState::Unavailable,
            false,
            NativeSecretStorageAction::UnlockRequired,
        ),
        NativeRecoveryPhase::Disabled => (
            NativeSecretStorageState::NotSetUp,
            false,
            NativeSecretStorageAction::BootstrapRequired,
        ),
        NativeRecoveryPhase::Incomplete => (
            NativeSecretStorageState::Locked,
            false,
            NativeSecretStorageAction::UnlockRequired,
        ),
        NativeRecoveryPhase::Enabled => (
            NativeSecretStorageState::Ready,
            true,
            NativeSecretStorageAction::None,
        ),
    };
    NativeSecretStorageStatus {
        session_generation,
        state,
        exists,
        unlocked,
        default_key_set,
        passphrase_configured,
        bootstrap_ready,
        missing_secrets,
        action,
    }
}

pub fn operation_result(
    outcome: NativeSecretStorageOutcome,
    recovery_document_saved: bool,
    status: NativeSecretStorageStatus,
) -> NativeSecretStorageOperationResult {
    NativeSecretStorageOperationResult {
        outcome,
        recovery_document_saved,
        recovery_document_name: recovery_document_saved.then_some(RECOVERY_DOCUMENT_NAME),
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_distinguishes_setup_unlock_and_ready() {
        let disabled = project_secret_storage_status(
            4,
            NativeRecoveryPhase::Disabled,
            false,
            false,
            false,
            false,
            vec![],
        );
        assert_eq!(disabled.state, NativeSecretStorageState::NotSetUp);
        assert_eq!(
            disabled.action,
            NativeSecretStorageAction::BootstrapRequired
        );
        assert!(!disabled.unlocked);

        let locked = project_secret_storage_status(
            4,
            NativeRecoveryPhase::Incomplete,
            true,
            true,
            true,
            true,
            vec![],
        );
        assert_eq!(locked.state, NativeSecretStorageState::Locked);
        assert_eq!(locked.action, NativeSecretStorageAction::UnlockRequired);

        let ready = project_secret_storage_status(
            4,
            NativeRecoveryPhase::Enabled,
            true,
            true,
            true,
            true,
            vec![],
        );
        assert_eq!(ready.state, NativeSecretStorageState::Ready);
        assert_eq!(ready.action, NativeSecretStorageAction::None);
        assert!(ready.unlocked);
    }

    #[test]
    fn status_and_operation_results_never_serialize_secret_material() {
        let status = project_secret_storage_status(
            8,
            NativeRecoveryPhase::Incomplete,
            true,
            true,
            true,
            true,
            vec![NativeMissingSecret::EncryptionBackup],
        );
        let result = operation_result(NativeSecretStorageOutcome::Complete, true, status);
        let json = serde_json::to_string(&result).unwrap().to_ascii_lowercase();
        for forbidden in [
            "recoverykey",
            "recovery_key",
            "privatekey",
            "private_key",
            "ciphertext",
            "passphrase\":\"",
            "secret_storage_key",
        ] {
            assert!(!json.contains(forbidden), "{json}");
        }
        assert!(json.contains("\"recoverydocumentsaved\":true"));
    }
}
