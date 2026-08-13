//! Unit tests for P6.9 room ops queue.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(matrix_room_ops_markers(), MATRIX_ROOM_OPS_MARKER);
}

#[test]
fn create_join_lifecycle() {
    let mut q = RoomOpsQueue::new(1);
    let create = q.enqueue_create(Some("Team".into())).unwrap().clone();
    assert_eq!(create.kind, RoomOpKind::Create);
    assert_eq!(create.state, RoomOpState::Pending);
    assert!(create.room_id.is_none());

    q.mark_in_flight(&create.local_op_id).unwrap();
    let done = q
        .mark_succeeded(&create.local_op_id, Some("!new:example.org".into()))
        .unwrap();
    assert_eq!(done.state, RoomOpState::Succeeded);
    assert_eq!(done.room_id.as_deref(), Some("!new:example.org"));

    let join = q.enqueue_join("!r:example.org").unwrap().clone();
    q.mark_in_flight(&join.local_op_id).unwrap();
    q.mark_succeeded(&join.local_op_id, None).unwrap();
    assert_eq!(
        q.get(&join.local_op_id).unwrap().state,
        RoomOpState::Succeeded
    );
}

#[test]
fn invite_kick_ban_unban_leave_forget() {
    let mut q = RoomOpsQueue::new(2);
    let invite = q
        .enqueue_invite("!r:example.org", "@u:example.org")
        .unwrap()
        .clone();
    assert_eq!(invite.target_user_id.as_deref(), Some("@u:example.org"));

    let kick = q
        .enqueue_kick("!r:example.org", "@u:example.org", Some("spam".into()))
        .unwrap()
        .clone();
    assert_eq!(kick.reason.as_deref(), Some("spam"));

    q.enqueue_ban("!r:example.org", "@u:example.org", None)
        .unwrap();
    q.enqueue_unban("!r:example.org", "@u:example.org").unwrap();
    q.enqueue_leave("!r:example.org", None).unwrap();
    q.enqueue_forget("!r:example.org").unwrap();
    assert_eq!(q.len(), 6);
    assert_eq!(q.list_for_room("!r:example.org").len(), 6);
}

#[test]
fn fail_retry_cancel_prune() {
    let mut q = RoomOpsQueue::new(3);
    let op = q.enqueue_join("!r:example.org").unwrap().clone();
    q.mark_in_flight(&op.local_op_id).unwrap();
    q.mark_failed(&op.local_op_id, "p6.9-hs-forbidden").unwrap();
    assert_eq!(
        q.get(&op.local_op_id).unwrap().failure_diagnostic_id,
        Some("p6.9-hs-forbidden")
    );
    q.retry(&op.local_op_id).unwrap();
    assert_eq!(q.get(&op.local_op_id).unwrap().state, RoomOpState::Pending);
    q.cancel(&op.local_op_id).unwrap();
    assert_eq!(q.prune_terminal(), 1);
    assert!(q.is_empty());
}

#[test]
fn validation_and_forbidden_reason() {
    let mut q = RoomOpsQueue::new(4);
    let err = q.enqueue_join("bad").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.9-invalid-room-id");
    let err = q.enqueue_invite("!r:example.org", "not-user").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.9-invalid-user-id");
    let err = q
        .enqueue_kick(
            "!r:example.org",
            "@u:example.org",
            Some("leaked access_token=x".into()),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.9-forbidden-reason");

    let create = q.enqueue_create(None).unwrap().clone();
    q.mark_in_flight(&create.local_op_id).unwrap();
    let err = q.mark_succeeded(&create.local_op_id, None).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.9-create-missing-room-id");
}

#[test]
fn retire_generation() {
    let mut q = RoomOpsQueue::new(5);
    q.enqueue_join("!r:example.org").unwrap();
    q.retire_generation(9);
    assert!(q.is_empty());
    assert_eq!(q.session_generation(), 9);
    assert_eq!(q.active_count(), 0);
}

#[test]
fn kind_labels() {
    for k in RoomOpKind::ALL {
        assert!(!k.as_str().is_empty());
    }
}
