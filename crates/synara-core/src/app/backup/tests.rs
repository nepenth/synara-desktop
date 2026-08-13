//! Unit tests for P8.5 backup / recovery flow.

use super::*;
use crate::dto::{BackupStatus, RecoveryStatus};
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_backup_markers(), MATRIX_BACKUP_MARKER);
}

#[test]
fn setup_succeed_path() {
    let mut flow = BackupRecoveryFlow::new(1);
    assert!(!flow.needs_attention()); // Unknown is not attention
    flow.apply_status(BackupStatus::Disabled, RecoveryStatus::NotSetup)
        .unwrap();
    assert!(flow.needs_attention());
    let op = flow.begin(BackupFlowKind::Setup).unwrap();
    assert!(flow.is_active());
    assert!(!flow.needs_attention()); // mid-flow
    flow.mark_awaiting_host(op).unwrap();
    flow.set_progress(op, 40).unwrap();
    flow.succeed(op, BackupStatus::Enabled, RecoveryStatus::Ready)
        .unwrap();
    assert_eq!(flow.phase(), BackupFlowPhase::Succeeded);
    assert_eq!(flow.backup_status(), BackupStatus::Enabled);
    assert_eq!(flow.recovery_status(), RecoveryStatus::Ready);
    assert!(!flow.needs_attention());
    flow.reset_to_idle().unwrap();
    assert_eq!(flow.phase(), BackupFlowPhase::Idle);
}

#[test]
fn restore_fail_cancel_stale_op() {
    let mut flow = BackupRecoveryFlow::new(1);
    let op = flow.begin(BackupFlowKind::Restore).unwrap();
    flow.fail(op, "p8.5-wrong-recovery-key").unwrap();
    assert_eq!(flow.phase(), BackupFlowPhase::Failed);
    assert_eq!(
        flow.failure_diagnostic_id(),
        Some("p8.5-wrong-recovery-key")
    );
    flow.reset_to_idle().unwrap();

    let op2 = flow.begin(BackupFlowKind::Repair).unwrap();
    flow.cancel(op2).unwrap();
    assert_eq!(flow.phase(), BackupFlowPhase::Cancelled);

    let err = flow.set_progress(op, 10).unwrap_err(); // stale
    assert_eq!(err.diagnostic_id(), "p8.5-stale-op-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn double_begin_and_progress_range() {
    let mut flow = BackupRecoveryFlow::new(1);
    let op = flow.begin(BackupFlowKind::Setup).unwrap();
    let err = flow.begin(BackupFlowKind::Restore).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.5-flow-already-active");
    let err = flow.set_progress(op, 101).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.5-progress-range");
}

#[test]
fn retire_generation() {
    let mut flow = BackupRecoveryFlow::new(2);
    flow.apply_status(BackupStatus::Enabled, RecoveryStatus::Ready)
        .unwrap();
    let _ = flow.begin(BackupFlowKind::Repair).unwrap();
    flow.retire_generation(3);
    assert_eq!(flow.session_generation(), 3);
    assert_eq!(flow.phase(), BackupFlowPhase::Idle);
    assert!(!flow.is_active());
}
