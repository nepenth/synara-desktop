//! Unit tests for P8.6 room-key transfer flow.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(matrix_room_keys_markers(), MATRIX_ROOM_KEYS_MARKER);
}

#[test]
fn export_success_path() {
    let mut flow = RoomKeyTransferFlow::new(1);
    let op = flow
        .begin(
            RoomKeyTransferKind::Export,
            Some("keys-2026-07-28.elek".into()),
        )
        .unwrap();
    assert_eq!(flow.phase(), RoomKeyTransferPhase::Preparing);
    flow.mark_in_flight(op).unwrap();
    flow.set_progress(op, 40).unwrap();
    flow.succeed(
        op,
        RoomKeyTransferOutcome {
            kind: RoomKeyTransferKind::Export,
            keys_processed: 12,
            rooms_touched: 3,
        },
    )
    .unwrap();
    assert_eq!(flow.phase(), RoomKeyTransferPhase::Succeeded);
    assert_eq!(flow.keys_processed(), 12);
    assert_eq!(flow.file_label(), Some("keys-2026-07-28.elek"));
    flow.reset_to_idle().unwrap();
    assert_eq!(flow.phase(), RoomKeyTransferPhase::Idle);
}

#[test]
fn import_fail_forbids_secret_diagnostics() {
    let mut flow = RoomKeyTransferFlow::new(1);
    let op = flow
        .begin(RoomKeyTransferKind::Import, Some("backup.elek".into()))
        .unwrap();
    flow.mark_in_flight(op).unwrap();
    let err = flow.fail(op, "leak-session_key").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.6-forbidden-diagnostic");
    flow.fail(op, "p8.6-wrong-passphrase").unwrap();
    assert_eq!(flow.phase(), RoomKeyTransferPhase::Failed);
    assert_eq!(flow.failure_diagnostic_id(), Some("p8.6-wrong-passphrase"));
}

#[test]
fn rejects_path_file_labels_and_busy() {
    let mut flow = RoomKeyTransferFlow::new(1);
    let err = flow
        .begin(
            RoomKeyTransferKind::Export,
            Some("/tmp/secret-keys.elek".into()),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.6-file-label-not-basename");
    let op = flow
        .begin(RoomKeyTransferKind::Export, Some("ok.elek".into()))
        .unwrap();
    let err = flow
        .begin(RoomKeyTransferKind::Import, Some("other.elek".into()))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.6-transfer-already-active");
    flow.cancel(op).unwrap();
    assert_eq!(flow.phase(), RoomKeyTransferPhase::Cancelled);
}

#[test]
fn stale_op_and_retire() {
    let mut flow = RoomKeyTransferFlow::new(1);
    let op = flow
        .begin(RoomKeyTransferKind::Import, Some("a.elek".into()))
        .unwrap();
    let err = flow.mark_in_flight(op + 1).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.6-stale-op-id");
    flow.retire_generation(9);
    assert_eq!(flow.session_generation(), 9);
    assert_eq!(flow.phase(), RoomKeyTransferPhase::Failed);
    assert_eq!(
        flow.failure_diagnostic_id(),
        Some("p8.6-stale-generation-cancelled")
    );
}
