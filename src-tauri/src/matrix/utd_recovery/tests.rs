//! Unit tests for P8.7 UTD recovery coordinator.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(matrix_utd_recovery_markers(), MATRIX_UTD_RECOVERY_MARKER);
}

#[test]
fn retry_succeed_clears_pending() {
    let mut c = UtdRecoveryCoordinator::new(1);
    let op = c
        .begin(
            "!r:example.org",
            UtdRecoveryKind::RetryDecrypt,
            vec!["$e1:example.org".into(), "$e2:example.org".into()],
        )
        .unwrap();
    assert_eq!(c.active_count(), 1);
    c.mark_in_flight("!r:example.org", op).unwrap();
    c.report_progress("!r:example.org", op, 1, 1).unwrap();
    c.succeed("!r:example.org", op, 2, 0).unwrap();
    let s = c.get("!r:example.org").unwrap();
    assert_eq!(s.phase, UtdRecoveryPhase::Succeeded);
    assert_eq!(s.recovered_count, 2);
    assert!(s.pending_event_ids.is_empty());
}

#[test]
fn history_recovery_partial() {
    let mut c = UtdRecoveryCoordinator::new(1);
    let op = c
        .begin(
            "!r:example.org",
            UtdRecoveryKind::EncryptedHistoryRecovery,
            vec![],
        )
        .unwrap();
    c.mark_in_flight("!r:example.org", op).unwrap();
    c.succeed("!r:example.org", op, 5, 3).unwrap();
    assert_eq!(
        c.get("!r:example.org").unwrap().phase,
        UtdRecoveryPhase::PartialSuccess
    );
    assert_eq!(c.get("!r:example.org").unwrap().still_utd_count, 3);
}

#[test]
fn fail_forbids_secrets_and_busy() {
    let mut c = UtdRecoveryCoordinator::new(1);
    let op = c
        .begin("!r:example.org", UtdRecoveryKind::RetryDecrypt, vec![])
        .unwrap();
    c.mark_in_flight("!r:example.org", op).unwrap();
    let err = c
        .fail("!r:example.org", op, "session_key-leak")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.7-forbidden-diagnostic");
    c.fail("!r:example.org", op, "p8.7-network").unwrap();
    let err = c
        .begin("!r:example.org", UtdRecoveryKind::RetryDecrypt, vec![])
        .unwrap(); // after fail, can begin again
    assert!(err > 0);
    // second concurrent room ok
    c.begin("!other:example.org", UtdRecoveryKind::RetryDecrypt, vec![])
        .unwrap();
    // busy same room
    c.mark_in_flight(
        "!other:example.org",
        c.get("!other:example.org").unwrap().op_id,
    )
    .unwrap();
    let err = c
        .begin("!other:example.org", UtdRecoveryKind::RetryDecrypt, vec![])
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.7-room-already-active");
}

#[test]
fn validation_and_retire() {
    let mut c = UtdRecoveryCoordinator::new(1);
    let err = c
        .begin("bad", UtdRecoveryKind::RetryDecrypt, vec![])
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.7-invalid-room-id");
    let err = c
        .begin(
            "!r:example.org",
            UtdRecoveryKind::RetryDecrypt,
            vec!["not-event".into()],
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.7-invalid-event-id");
    c.begin("!r:example.org", UtdRecoveryKind::RetryDecrypt, vec![])
        .unwrap();
    c.retire_generation(4);
    assert!(c.is_empty());
    assert_eq!(c.session_generation(), 4);
}
