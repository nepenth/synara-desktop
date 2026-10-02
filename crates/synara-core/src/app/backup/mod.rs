//! P8.5 — Key backup / recovery setup-restore-repair foundation (harness).
//!
//! Pure flow state machine plus live status and restore. **Never stores
//! recovery keys or secrets.** Setup/restore/repair SDK semantics are shared across native platforms.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.5-backup-recovery.md`

mod error;
mod flow;
mod live;
mod status;

pub use error::BackupError;
pub use flow::{BackupFlowKind, BackupFlowPhase, BackupRecoveryFlow};
pub use live::{repair, restore, restore_operation, setup, status, MatrixRestoreBackupResult};
pub use status::{
    project_backup_status, NativeBackupAction, NativeBackupAvailability, NativeBackupDeviceState,
    NativeBackupEnginePhase, NativeBackupOperationOutcome, NativeBackupOperationResult,
    NativeBackupRecoveryPhase, NativeBackupRecoveryState, NativeBackupStatus,
    ServerBackupProjection,
};

#[cfg(test)]
mod tests;
