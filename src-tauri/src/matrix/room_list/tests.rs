//! Unit tests for P4.2 room-list snapshot/delta projection.

use super::*;
use crate::matrix::dto::{Membership, RoomSummary};
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn room(id: &str, name: &str) -> RoomSummary {
    RoomSummaryBuilder::new(id)
        .name(name)
        .build()
        .expect("room summary")
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_room_list_markers(), MATRIX_ROOM_LIST_MARKER);
}

#[test]
fn snapshot_then_ordered_deltas_reconstruct() {
    let a = room("!a:example.org", "Alpha");
    let b = room("!b:example.org", "Beta");
    let c = room("!c:example.org", "Gamma");

    let mut proj = RoomListProjection::new(7);
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 7,
        sequence: 1,
        rooms: vec![a.clone(), b.clone()],
    })
    .unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.last_sequence(), 1);

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 7,
        sequence: 2,
        ops: vec![
            RoomListDeltaOp::PushBack { room: c.clone() },
            RoomListDeltaOp::Set {
                index: 0,
                room: RoomSummaryBuilder::new("!a:example.org")
                    .name("Alpha*")
                    .unread(2, 1)
                    .build()
                    .unwrap(),
            },
        ],
    })
    .unwrap();
    assert_eq!(proj.len(), 3);
    assert_eq!(proj.rooms()[0].name.as_deref(), Some("Alpha*"));
    assert_eq!(proj.rooms()[0].unread_count, 2);
    assert_eq!(proj.rooms()[2].room_id.as_str(), "!c:example.org");
    assert_eq!(proj.last_sequence(), 2);
    assert!(!proj.resync_required());
}

#[test]
fn sequence_gap_requires_resync_then_reset_recovers() {
    let mut proj = RoomListProjection::new(1);
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 1,
        sequence: 5,
        rooms: vec![room("!a:example.org", "A")],
    })
    .unwrap();

    let err = proj
        .apply_batch(RoomListDeltaBatch {
            session_generation: 1,
            sequence: 7, // gap: expected 6
            ops: vec![RoomListDeltaOp::PushBack {
                room: room("!b:example.org", "B"),
            }],
        })
        .unwrap_err();
    assert!(err.requires_resync());
    assert_eq!(err.diagnostic_id(), "p4.2-sequence-gap");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert!(proj.resync_required());

    // Non-reset while resync required is refused.
    let err2 = proj
        .apply_batch(RoomListDeltaBatch {
            session_generation: 1,
            sequence: 6,
            ops: vec![RoomListDeltaOp::Clear],
        })
        .unwrap_err();
    assert_eq!(err2.diagnostic_id(), "p4.2-resync-pending");

    // Reset recovers.
    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 10,
        ops: vec![RoomListDeltaOp::Reset {
            rooms: vec![room("!z:example.org", "Z")],
        }],
    })
    .unwrap();
    assert!(!proj.resync_required());
    assert_eq!(proj.len(), 1);
    assert_eq!(proj.rooms()[0].name.as_deref(), Some("Z"));
    assert_eq!(proj.last_sequence(), 10);
}

#[test]
fn stale_generation_is_rejected() {
    let mut proj = RoomListProjection::new(2);
    let err = proj
        .apply_snapshot(RoomListSnapshot {
            session_generation: 3,
            sequence: 1,
            rooms: vec![],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.2-snapshot-stale-generation");
    assert_eq!(
        err.category(),
        MatrixIpcErrorCategory::StaleSessionGeneration
    );
}

#[test]
fn oob_ops_mark_resync() {
    let mut proj = RoomListProjection::new(1);
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 1,
        sequence: 1,
        rooms: vec![room("!a:example.org", "A")],
    })
    .unwrap();

    let err = proj
        .apply_batch(RoomListDeltaBatch {
            session_generation: 1,
            sequence: 2,
            ops: vec![RoomListDeltaOp::Remove { index: 5 }],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.2-remove-oob");
    assert!(proj.resync_required());
}

#[test]
fn move_insert_truncate_clear_ops() {
    let mut proj = RoomListProjection::new(1);
    let a = room("!a:example.org", "A");
    let b = room("!b:example.org", "B");
    let c = room("!c:example.org", "C");
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 1,
        sequence: 1,
        rooms: vec![a.clone(), b.clone(), c.clone()],
    })
    .unwrap();

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 2,
        ops: vec![RoomListDeltaOp::Move { from: 0, to: 2 }],
    })
    .unwrap();
    assert_eq!(
        proj.rooms()
            .iter()
            .map(|r| r.room_id.as_str())
            .collect::<Vec<_>>(),
        vec!["!b:example.org", "!c:example.org", "!a:example.org"]
    );

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 3,
        ops: vec![RoomListDeltaOp::Truncate { len: 2 }],
    })
    .unwrap();
    assert_eq!(proj.len(), 2);

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 4,
        ops: vec![RoomListDeltaOp::Insert {
            index: 1,
            room: room("!d:example.org", "D"),
        }],
    })
    .unwrap();
    assert_eq!(proj.rooms()[1].name.as_deref(), Some("D"));

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 5,
        ops: vec![RoomListDeltaOp::Clear],
    })
    .unwrap();
    assert!(proj.is_empty());
}

#[test]
fn reconstruct_helper_matches_manual_apply() {
    let snap = RoomListSnapshot {
        session_generation: 4,
        sequence: 1,
        rooms: vec![room("!a:example.org", "A")],
    };
    let batches = vec![RoomListDeltaBatch {
        session_generation: 4,
        sequence: 2,
        ops: vec![RoomListDeltaOp::Append {
            rooms: vec![room("!b:example.org", "B")],
        }],
    }];
    let proj = reconstruct(4, snap, &batches).unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.last_sequence(), 2);
}

#[test]
fn summary_builder_membership_and_privacy() {
    let s = RoomSummaryBuilder::new("!dm:example.org")
        .name("Bob")
        .membership(Membership::Invite)
        .direct(true)
        .encrypted(true)
        .unread(3, 1)
        .marked_unread(true)
        .last_activity_ts(1_700_000_000_000)
        .build()
        .unwrap();
    assert_eq!(s.membership, Membership::Invite);
    assert!(s.is_direct);
    assert!(s.is_encrypted);
    assert_eq!(s.unread_count, 3);
    let dbg = format!("{s:?}");
    assert!(!dbg.contains("access_token"));
    assert!(!dbg.contains("syt_"));
}

#[test]
fn snapshot_into_reset_batch() {
    let snap = RoomListSnapshot {
        session_generation: 9,
        sequence: 3,
        rooms: vec![room("!a:example.org", "A")],
    };
    let batch = snap.into_reset_batch();
    assert_eq!(batch.session_generation, 9);
    assert_eq!(batch.sequence, 3);
    assert!(matches!(batch.ops[0], RoomListDeltaOp::Reset { .. }));
}

#[test]
fn delta_op_names_are_stable() {
    assert_eq!(RoomListDeltaOp::Clear.op_name(), "clear");
    assert_eq!(RoomListDeltaOp::Remove { index: 0 }.op_name(), "remove");
}
