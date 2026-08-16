//! P8.5 — Key backup / recovery setup-restore-repair foundation (harness).
//!
//! Pure flow state machine plus live status reads. **Never stores recovery
//! keys or secrets.** Setup/restore/repair stay in the desktop shell.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.5-backup-recovery.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod flow;
mod live;
mod status;

pub use error::BackupError;
pub use flow::{BackupFlowKind, BackupFlowPhase, BackupRecoveryFlow};
pub use live::status;
pub use status::{
    project_backup_status, NativeBackupAction, NativeBackupAvailability, NativeBackupDeviceState,
    NativeBackupEnginePhase, NativeBackupOperationOutcome, NativeBackupOperationResult,
    NativeBackupRecoveryPhase, NativeBackupRecoveryState, NativeBackupStatus,
    ServerBackupProjection,
};

/// Static marker for link / schema smoke.
pub const MATRIX_BACKUP_MARKER: &str = "matrix-backup-p8.5";

/// Touch backup paths so they remain linked in non-test builds.
pub fn matrix_backup_markers() -> &'static str {
    let flow = BackupRecoveryFlow::new(0);
    debug_assert!(!flow.is_active());
    debug_assert_eq!(flow.phase(), BackupFlowPhase::Idle);
    debug_assert_eq!(MATRIX_BACKUP_MARKER, "matrix-backup-p8.5");
    MATRIX_BACKUP_MARKER
}

#[cfg(test)]
mod tests;
