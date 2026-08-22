//! Unit tests for P5.1 timeline registry + P5.2 snapshot/diff projection.

use super::*;
use crate::matrix::dto::{TimelineItem, TimelineMessageItem};
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn msg(id: &str, body: &str) -> TimelineItem {
    TimelineItem::Message(TimelineMessageItem {
        item_id: id.into(),
        event_id: id.into(),
        room_id: "!room:example.org".into(),
        sender: "@alice:example.org".into(),
        origin_server_ts: 1_720_000_000_000,
        body: body.into(),
        msgtype: Some("m.text".into()),
        relates_to: None,
        local_echo_state: None,
        is_edited: None,
        is_redacted: None,
        thread_root_id: None,
    })
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_timeline_markers(), MATRIX_TIMELINE_MARKER);
}

#[test]
fn open_mark_live_close_dispose() {
    let mut reg = TimelineRegistry::new(3);
    let key = TimelineKey::main("!room:example.org").unwrap();
    let e = reg.open(key.clone()).unwrap();
    assert_eq!(e.lifecycle, TimelineLifecycle::Opening);
    assert_eq!(e.session_generation, 3);
    assert_eq!(reg.active_count(), 1);

    reg.mark_live(&key).unwrap();
    assert_eq!(reg.get(&key).unwrap().lifecycle, TimelineLifecycle::Live);

    reg.close(&key).unwrap();
    assert_eq!(reg.get(&key).unwrap().lifecycle, TimelineLifecycle::Closed);
    assert_eq!(reg.active_count(), 0);

    // Reopen after close.
    reg.open(key.clone()).unwrap();
    reg.mark_live(&key).unwrap();

    reg.dispose(&key).unwrap();
    assert!(reg.get(&key).is_none());
    assert!(reg.is_empty());
}

#[test]
fn already_open_rejected() {
    let mut reg = TimelineRegistry::new(1);
    let key = TimelineKey::main("!r:example.org").unwrap();
    reg.open(key.clone()).unwrap();
    let err = reg.open(key).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.1-timeline-already-open");
}

#[test]
fn thread_key_distinct_from_main() {
    let mut reg = TimelineRegistry::new(1);
    let main = TimelineKey::main("!r:example.org").unwrap();
    let thr = TimelineKey::thread("!r:example.org", "$root").unwrap();
    reg.open(main.clone()).unwrap();
    reg.open(thr.clone()).unwrap();
    assert_eq!(reg.len(), 2);
    reg.mark_live(&main).unwrap();
    reg.mark_live(&thr).unwrap();
    assert_eq!(reg.active_count(), 2);
}

#[test]
fn retire_generation_closes_active() {
    let mut reg = TimelineRegistry::new(1);
    let key = TimelineKey::main("!r:example.org").unwrap();
    reg.open(key.clone()).unwrap();
    reg.mark_live(&key).unwrap();
    reg.retire_generation(2);
    assert_eq!(reg.session_generation(), 2);
    assert_eq!(reg.get(&key).unwrap().lifecycle, TimelineLifecycle::Closed);
    assert_eq!(reg.active_count(), 0);
    // Can open again under new generation.
    reg.open(key.clone()).unwrap();
    assert_eq!(reg.get(&key).unwrap().session_generation, 2);
}

#[test]
fn mark_failed_sets_diagnostic() {
    let mut reg = TimelineRegistry::new(1);
    let key = TimelineKey::main("!r:example.org").unwrap();
    reg.open(key.clone()).unwrap();
    reg.mark_failed(&key, "p5.1-attach-failed").unwrap();
    let e = reg.get(&key).unwrap();
    assert_eq!(e.lifecycle, TimelineLifecycle::Failed);
    assert_eq!(e.failure_diagnostic_id, Some("p5.1-attach-failed"));
}

#[test]
fn invalid_room_id_rejected() {
    let err = TimelineKey::main("not-a-room").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.1-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert!(!err.to_string().contains("access_token"));
}

#[test]
fn clear_wipes_registry() {
    let mut reg = TimelineRegistry::new(1);
    reg.open(TimelineKey::main("!a:example.org").unwrap())
        .unwrap();
    reg.clear();
    assert!(reg.is_empty());
}

// --- P5.2 projection ---

#[test]
fn p5_2_delta_op_names_are_stable() {
    assert_eq!(TimelineDeltaOp::Reset { items: vec![] }.op_name(), "reset");
    assert_eq!(
        TimelineDeltaOp::Append { items: vec![] }.op_name(),
        "append"
    );
    assert_eq!(TimelineDeltaOp::Clear.op_name(), "clear");
    assert_eq!(
        TimelineDeltaOp::PushFront {
            item: msg("$a", "a")
        }
        .op_name(),
        "push_front"
    );
    assert_eq!(
        TimelineDeltaOp::PushBack {
            item: msg("$b", "b")
        }
        .op_name(),
        "push_back"
    );
    assert_eq!(
        TimelineDeltaOp::Insert {
            index: 0,
            item: msg("$c", "c")
        }
        .op_name(),
        "insert"
    );
    assert_eq!(
        TimelineDeltaOp::Set {
            index: 0,
            item: msg("$d", "d")
        }
        .op_name(),
        "set"
    );
    assert_eq!(TimelineDeltaOp::Remove { index: 0 }.op_name(), "remove");
    assert_eq!(TimelineDeltaOp::Truncate { len: 0 }.op_name(), "truncate");
    assert_eq!(TimelineDeltaOp::Move { from: 0, to: 1 }.op_name(), "move");
}

#[test]
fn p5_2_snapshot_then_ordered_deltas_reconstruct() {
    let mut proj = TimelineProjection::new(7);
    proj.apply_snapshot(TimelineSnapshot {
        session_generation: 7,
        sequence: 1,
        items: vec![msg("$1", "one"), msg("$2", "two")],
    })
    .unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.last_sequence(), 1);

    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 7,
        sequence: 2,
        ops: vec![TimelineDeltaOp::PushBack {
            item: msg("$3", "three"),
        }],
    })
    .unwrap();
    assert_eq!(proj.len(), 3);
    assert_eq!(proj.items()[2].item_id(), "$3");

    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 7,
        sequence: 3,
        ops: vec![
            TimelineDeltaOp::Set {
                index: 0,
                item: msg("$1", "one-edited"),
            },
            TimelineDeltaOp::Remove { index: 1 },
        ],
    })
    .unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.items()[0].item_id(), "$1");
    match &proj.items()[0] {
        TimelineItem::Message(m) => assert_eq!(m.body, "one-edited"),
        other => panic!("expected message, got {}", other.kind()),
    }
}

#[test]
fn p5_2_sequence_gap_requires_resync_then_reset_recovers() {
    let mut proj = TimelineProjection::new(1);
    proj.apply_snapshot(TimelineSnapshot {
        session_generation: 1,
        sequence: 1,
        items: vec![msg("$1", "a")],
    })
    .unwrap();

    let err = proj
        .apply_batch(TimelineDeltaBatch {
            session_generation: 1,
            sequence: 3, // gap: expected 2
            ops: vec![TimelineDeltaOp::PushBack {
                item: msg("$2", "b"),
            }],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.2-sequence-gap");
    assert!(err.requires_resync());
    assert!(proj.resync_required());

    // Non-reset while resync pending is rejected.
    let err2 = proj
        .apply_batch(TimelineDeltaBatch {
            session_generation: 1,
            sequence: 2,
            ops: vec![TimelineDeltaOp::Clear],
        })
        .unwrap_err();
    assert_eq!(err2.diagnostic_id(), "p5.2-resync-pending");

    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 1,
        sequence: 10,
        ops: vec![TimelineDeltaOp::Reset {
            items: vec![msg("$z", "recovered")],
        }],
    })
    .unwrap();
    assert!(!proj.resync_required());
    assert_eq!(proj.len(), 1);
    assert_eq!(proj.last_sequence(), 10);
}

#[test]
fn p5_2_stale_generation_is_rejected() {
    let mut proj = TimelineProjection::new(2);
    let err = proj
        .apply_snapshot(TimelineSnapshot {
            session_generation: 1,
            sequence: 0,
            items: vec![],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.2-snapshot-stale-generation");
    assert_eq!(
        err.category(),
        MatrixIpcErrorCategory::StaleSessionGeneration
    );
}

#[test]
fn p5_2_move_insert_truncate_clear_ops() {
    let mut proj = TimelineProjection::new(1);
    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 1,
        sequence: 1,
        ops: vec![TimelineDeltaOp::Reset {
            items: vec![msg("$a", "a"), msg("$b", "b"), msg("$c", "c")],
        }],
    })
    .unwrap();

    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 1,
        sequence: 2,
        ops: vec![
            TimelineDeltaOp::Move { from: 0, to: 2 },
            TimelineDeltaOp::Insert {
                index: 0,
                item: msg("$z", "z"),
            },
            TimelineDeltaOp::Truncate { len: 3 },
        ],
    })
    .unwrap();
    assert_eq!(proj.len(), 3);
    assert_eq!(proj.items()[0].item_id(), "$z");

    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 1,
        sequence: 3,
        ops: vec![TimelineDeltaOp::Clear],
    })
    .unwrap();
    assert!(proj.is_empty());
}

#[test]
fn p5_2_oob_ops_mark_resync() {
    let mut proj = TimelineProjection::new(1);
    proj.apply_batch(TimelineDeltaBatch {
        session_generation: 1,
        sequence: 1,
        ops: vec![TimelineDeltaOp::Reset {
            items: vec![msg("$a", "a")],
        }],
    })
    .unwrap();
    let err = proj
        .apply_batch(TimelineDeltaBatch {
            session_generation: 1,
            sequence: 2,
            ops: vec![TimelineDeltaOp::Remove { index: 9 }],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.2-remove-oob");
    assert!(proj.resync_required());
}

#[test]
fn p5_2_snapshot_into_reset_batch() {
    let snap = TimelineSnapshot {
        session_generation: 4,
        sequence: 5,
        items: vec![msg("$1", "hi")],
    };
    let batch = snap.into_reset_batch();
    assert_eq!(batch.session_generation, 4);
    assert_eq!(batch.sequence, 5);
    assert_eq!(batch.ops.len(), 1);
    assert_eq!(batch.ops[0].op_name(), "reset");
}

#[test]
fn p5_2_reconstruct_helper_matches_manual_apply() {
    let snap = TimelineSnapshot {
        session_generation: 1,
        sequence: 1,
        items: vec![msg("$1", "a")],
    };
    let batches = vec![TimelineDeltaBatch {
        session_generation: 1,
        sequence: 2,
        ops: vec![TimelineDeltaOp::Append {
            items: vec![msg("$2", "b")],
        }],
    }];
    let proj = reconstruct(1, snap, &batches).unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.last_sequence(), 2);
}

// --- P5.3 pagination ---

#[test]
fn p5_3_begin_complete_backwards_page() {
    let key = TimelineKey::main("!room:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 3);
    assert_eq!(pag.session_generation(), 3);
    assert!(!pag.any_in_flight());

    pag.begin(PaginationRequest::backwards(50)).unwrap();
    assert!(pag.any_in_flight());
    assert_eq!(
        pag.status(PaginationDirection::Backwards).phase,
        PaginationPhase::InFlight
    );

    pag.complete(PaginationOutcome {
        direction: PaginationDirection::Backwards,
        items_applied: 20,
        exhausted: false,
    })
    .unwrap();
    let st = pag.status(PaginationDirection::Backwards);
    assert_eq!(st.phase, PaginationPhase::Idle);
    assert_eq!(st.pages_completed, 1);
    assert_eq!(st.items_loaded, 20);
    assert!(!pag.any_in_flight());
}

#[test]
fn p5_3_exhausted_rejects_further_begin() {
    let key = TimelineKey::main("!room:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 1);
    pag.begin(PaginationRequest::backwards(30)).unwrap();
    pag.complete(PaginationOutcome {
        direction: PaginationDirection::Backwards,
        items_applied: 0,
        exhausted: true,
    })
    .unwrap();
    assert_eq!(
        pag.status(PaginationDirection::Backwards).phase,
        PaginationPhase::Exhausted
    );
    let err = pag.begin(PaginationRequest::backwards(30)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.3-pagination-exhausted");
}

#[test]
fn p5_3_double_begin_rejected() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 1);
    pag.begin(PaginationRequest::backwards(10)).unwrap();
    let err = pag.begin(PaginationRequest::backwards(10)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.3-pagination-already-in-flight");
}

#[test]
fn p5_3_fail_then_clear_and_retry() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 1);
    pag.begin(PaginationRequest::forwards(5)).unwrap();
    pag.fail(PaginationDirection::Forwards, "p5.3-network-failed")
        .unwrap();
    let st = pag.status(PaginationDirection::Forwards);
    assert_eq!(st.phase, PaginationPhase::Failed);
    assert_eq!(st.failure_diagnostic_id, Some("p5.3-network-failed"));
    assert!(!st.failure_diagnostic_id.unwrap().contains("access_token"));

    pag.clear_failure(PaginationDirection::Forwards).unwrap();
    pag.begin(PaginationRequest::forwards(5)).unwrap();
    pag.complete(PaginationOutcome {
        direction: PaginationDirection::Forwards,
        items_applied: 3,
        exhausted: false,
    })
    .unwrap();
    assert_eq!(pag.status(PaginationDirection::Forwards).items_loaded, 3);
}

#[test]
fn p5_3_invalid_limit_rejected() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 1);
    let err = pag.begin(PaginationRequest::backwards(0)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.3-invalid-page-limit");
    let err = pag.begin(PaginationRequest::backwards(101)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.3-invalid-page-limit");
}

#[test]
fn p5_3_retire_generation_cancels_in_flight() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 1);
    pag.begin(PaginationRequest::backwards(20)).unwrap();
    pag.retire_generation(2);
    assert_eq!(pag.session_generation(), 2);
    let st = pag.status(PaginationDirection::Backwards);
    assert_eq!(st.phase, PaginationPhase::Failed);
    assert_eq!(
        st.failure_diagnostic_id,
        Some("p5.3-stale-generation-cancelled")
    );
}

#[test]
fn p5_3_directions_independent() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut pag = TimelinePagination::new(key, 1);
    pag.begin(PaginationRequest::backwards(10)).unwrap();
    // Forwards still idle and startable.
    pag.begin(PaginationRequest::forwards(10)).unwrap();
    assert!(pag.any_in_flight());
    pag.complete(PaginationOutcome {
        direction: PaginationDirection::Backwards,
        items_applied: 5,
        exhausted: false,
    })
    .unwrap();
    assert!(pag.any_in_flight()); // forwards still in flight
    pag.complete(PaginationOutcome {
        direction: PaginationDirection::Forwards,
        items_applied: 2,
        exhausted: true,
    })
    .unwrap();
    assert!(!pag.any_in_flight());
    assert_eq!(
        pag.status(PaginationDirection::Forwards).phase,
        PaginationPhase::Exhausted
    );
}

// --- P5.10 UTD / decryption update propagation ---

fn enc(
    item: &str,
    event: &str,
    room: &str,
) -> crate::matrix::dto::TimelineEncryptedUnavailableItem {
    crate::matrix::dto::TimelineEncryptedUnavailableItem {
        item_id: item.into(),
        event_id: event.into(),
        room_id: room.into(),
        reason: Some("missing_keys".into()),
    }
}

#[test]
fn p5_10_mark_retry_decrypt_flow() {
    let mut idx = UtdIndex::new(2);
    let u = idx
        .mark_unavailable(
            enc("i1", "$e1:example.org", "!r:example.org"),
            UtdReasonCode::MissingKeys,
        )
        .unwrap();
    assert!(matches!(u, UtdUpdate::MarkedUnavailable { .. }));
    assert_eq!(idx.active_utd_count(), 1);

    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org").unwrap().phase,
        UtdPhase::RetryPending
    );
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org")
            .unwrap()
            .retry_count,
        1
    );

    let u = idx
        .mark_decrypted("!r:example.org", "$e1:example.org")
        .unwrap();
    assert!(matches!(u, UtdUpdate::Decrypted { .. }));
    assert_eq!(idx.active_utd_count(), 0);
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org").unwrap().phase,
        UtdPhase::Decrypted
    );
}

#[test]
fn p5_10_retry_failed_then_permanent() {
    let mut idx = UtdIndex::new(1);
    idx.mark_unavailable(
        enc("i1", "$e1:example.org", "!r:example.org"),
        UtdReasonCode::Historical,
    )
    .unwrap();
    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    idx.retry_failed("!r:example.org", "$e1:example.org", "p5.10-key-not-yet")
        .unwrap();
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org").unwrap().phase,
        UtdPhase::UnableToDecrypt
    );

    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    let u = idx
        .mark_permanent_failure("!r:example.org", "$e1:example.org", "p5.10-unrecoverable")
        .unwrap();
    assert!(matches!(u, UtdUpdate::PermanentFailure { .. }));
    let err = idx
        .begin_retry("!r:example.org", "$e1:example.org")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-permanent-no-retry");
}

#[test]
fn p5_10_forbidden_reason_and_diagnostic() {
    let mut idx = UtdIndex::new(1);
    let mut bad = enc("i1", "$e1:example.org", "!r:example.org");
    bad.reason = Some("session_key=abc".into());
    let err = idx.mark_unavailable(bad, UtdReasonCode::Other).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-forbidden-reason");

    idx.mark_unavailable(
        enc("i1", "$e1:example.org", "!r:example.org"),
        UtdReasonCode::MissingKeys,
    )
    .unwrap();
    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    let err = idx
        .retry_failed("!r:example.org", "$e1:example.org", "leak-access_token")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-forbidden-diagnostic");
}

#[test]
fn p5_10_list_active_and_gc_and_retire() {
    let mut idx = UtdIndex::new(1);
    idx.mark_unavailable(
        enc("i1", "$a:example.org", "!r:example.org"),
        UtdReasonCode::MissingKeys,
    )
    .unwrap();
    idx.mark_unavailable(
        enc("i2", "$b:example.org", "!r:example.org"),
        UtdReasonCode::Withheld,
    )
    .unwrap();
    idx.mark_unavailable(
        enc("i3", "$c:example.org", "!other:example.org"),
        UtdReasonCode::Other,
    )
    .unwrap();
    assert_eq!(idx.list_active_for_room("!r:example.org").len(), 2);

    idx.mark_decrypted("!r:example.org", "$a:example.org")
        .unwrap();
    assert_eq!(idx.gc_decrypted(), 1);
    assert!(idx.get("!r:example.org", "$a:example.org").is_none());

    idx.retire_generation(9);
    assert_eq!(idx.session_generation(), 9);
    assert!(idx.is_empty());
}

#[test]
fn p5_10_invalid_ids() {
    let mut idx = UtdIndex::new(1);
    let err = idx
        .mark_unavailable(
            enc("i1", "not-event", "!r:example.org"),
            UtdReasonCode::Other,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-invalid-event-id");
}

// --- P5.4 focus / event-context opening ---

#[test]
fn p5_4_open_focused_settle_ready() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 1);
    assert!(focus.is_live());
    assert_eq!(focus.phase(), NavigationPhase::Idle);

    focus
        .begin_open(FocusOpenRequest::focused("$evt1:example.org"))
        .unwrap();
    assert_eq!(focus.phase(), NavigationPhase::LoadingContext);
    assert_eq!(focus.mode().as_kind_str(), "focused");
    assert_eq!(focus.highlight_event_id(), Some("$evt1:example.org"));
    assert!(focus.is_busy());

    focus
        .complete_open(FocusOpenOutcome {
            items_applied: 40,
            target_found: true,
            at_live_bottom: false,
        })
        .unwrap();
    assert_eq!(focus.phase(), NavigationPhase::SettlingLayout);
    assert_eq!(focus.opens_completed(), 1);

    focus.confirm_ready().unwrap();
    assert_eq!(focus.phase(), NavigationPhase::BottomConfirmed);
    assert!(!focus.is_live());
}

#[test]
fn p5_4_open_unread_then_jump_latest() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 2);
    focus
        .begin_open(FocusOpenRequest::unread("$mark:example.org"))
        .unwrap();
    focus
        .complete_open(FocusOpenOutcome {
            items_applied: 20,
            target_found: true,
            at_live_bottom: false,
        })
        .unwrap();
    focus.confirm_ready().unwrap();
    assert_eq!(focus.mode().as_kind_str(), "unread");

    focus.begin_jump_latest().unwrap();
    assert_eq!(focus.phase(), NavigationPhase::RebindingLive);
    focus
        .complete_open(FocusOpenOutcome {
            items_applied: 10,
            target_found: true,
            at_live_bottom: true,
        })
        .unwrap();
    assert!(focus.is_live());
    assert!(focus.highlight_event_id().is_none());
    focus.confirm_ready().unwrap();
    assert_eq!(focus.phase(), NavigationPhase::BottomConfirmed);
}

#[test]
fn p5_4_target_not_found_errors() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 1);
    focus
        .begin_open(FocusOpenRequest::focused("$missing:example.org"))
        .unwrap();
    let err = focus
        .complete_open(FocusOpenOutcome {
            items_applied: 0,
            target_found: false,
            at_live_bottom: false,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.4-target-not-found");
    assert_eq!(focus.phase(), NavigationPhase::Error);
    focus.clear_failure().unwrap();
    assert_eq!(focus.phase(), NavigationPhase::Idle);
}

#[test]
fn p5_4_busy_rejects_double_open() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 1);
    focus.begin_open(FocusOpenRequest::live()).unwrap();
    let err = focus
        .begin_open(FocusOpenRequest::focused("$e:example.org"))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.4-navigation-busy");
}

#[test]
fn p5_4_invalid_event_id_and_window() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 1);
    let err = focus
        .begin_open(FocusOpenRequest::focused("not-an-event"))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.4-invalid-event-id");
    let err = focus.begin_open(FocusOpenRequest::focused("")).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.4-invalid-event-id");
    let err = focus
        .begin_open(
            FocusOpenRequest::focused("$ok:example.org").with_window(ContextWindow {
                before: 101,
                after: 0,
            }),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.4-invalid-context-window");
}

#[test]
fn p5_4_fail_rejects_secret_diagnostics() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 1);
    focus
        .begin_open(FocusOpenRequest::focused("$e:example.org"))
        .unwrap();
    let err = focus.fail("leak-access_token").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.4-forbidden-diagnostic");
    focus.fail("p5.4-homeserver-timeout").unwrap();
    assert_eq!(focus.phase(), NavigationPhase::Error);
    assert_eq!(
        focus.failure_diagnostic_id(),
        Some("p5.4-homeserver-timeout")
    );
    assert!(!focus
        .failure_diagnostic_id()
        .unwrap()
        .contains("access_token"));
}

#[test]
fn p5_4_retire_generation_cancels_in_flight() {
    let key = TimelineKey::main("!r:example.org").unwrap();
    let mut focus = TimelineFocus::new(key, 1);
    focus
        .begin_open(FocusOpenRequest::focused("$e:example.org"))
        .unwrap();
    focus.retire_generation(9);
    assert_eq!(focus.session_generation(), 9);
    assert_eq!(focus.phase(), NavigationPhase::Error);
    assert_eq!(
        focus.failure_diagnostic_id(),
        Some("p5.4-stale-generation-cancelled")
    );
    assert!(focus.is_live());
}

// ---- SNC-P1-5c pure-module mirrors ------------------------------
// The pure timeline modules (actions, composer, media, view) moved into
// `synara_core::app::timeline` with their internal unit tests. These mirrors
// exercise the same product contracts through the src-tauri adapter
// re-exports so the desktop test count stays identical to the pre-move
// baseline (same shape as SNC-P1-5a/b mirrors).

mod actions_pure {
    use super::*;

    #[test]
    fn formatted_body_attaches_only_when_it_differs_from_plain_text() {
        assert!(!should_attach_formatted_body("hello", Some("hello")));
        assert!(!should_attach_formatted_body("hello", Some("  ")));
        assert!(should_attach_formatted_body("hello", Some("<p>hello</p>")));
    }

    #[test]
    fn forward_plain_and_quote_bodies_attribute_the_source_sender() {
        assert_eq!(
            format_forwarded_plain_body("@alice:example.org", "hello", false),
            "Forwarded from @alice:example.org\n\nhello"
        );
        assert_eq!(
            format_forwarded_plain_body("@alice:example.org", "hello\nthere", true),
            "> <@alice:example.org>\n> hello\n> there"
        );
    }

    #[test]
    fn action_request_schemas_stay_room_addressed() {
        let edit: NativeTimelineEditTextRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$edit:example.org",
            "body": "updated"
        }))
        .unwrap();
        assert_eq!(edit.event_id, "$edit:example.org");

        let media: NativeTimelineForwardMediaRequest = serde_json::from_value(serde_json::json!({
            "sourceRoomId": "!source:example.org",
            "eventId": "$media:example.org",
            "targetRoomId": "!target:example.org"
        }))
        .unwrap();
        assert_eq!(media.event_id, "$media:example.org");
        assert_eq!(
            format_forwarded_media_body("@alice:example.org", "photo.jpg"),
            "Forwarded from @alice:example.org\n\nphoto.jpg"
        );

        let vote: NativeTimelinePollVoteRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$poll:example.org",
            "answerIds": ["a1", "a2"]
        }))
        .unwrap();
        assert_eq!(vote.answer_ids, vec!["a1", "a2"]);

        let decline: NativeTimelineCallDeclineRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$rtc:example.org"
        }))
        .unwrap();
        assert_eq!(decline.event_id, "$rtc:example.org");

        let redact: NativeTimelineRedactRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$redact:example.org",
            "reason": "spam"
        }))
        .unwrap();
        assert_eq!(redact.reason.as_deref(), Some("spam"));

        let forward: NativeTimelineForwardTextRequest = serde_json::from_value(serde_json::json!({
            "sourceRoomId": "!source:example.org",
            "eventId": "$fwd:example.org",
            "targetRoomId": "!target:example.org",
            "asQuote": true
        }))
        .unwrap();
        assert!(forward.as_quote);

        let readback = NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::EditText,
            room_id: "!room:example.org".into(),
            event_id: "$new:example.org".into(),
            status: "sent".into(),
        };
        let json = serde_json::to_value(readback).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["action"], "edit_text");
        assert_eq!(json["status"], "sent");
    }
}

mod composer_pure {
    use super::*;

    #[test]
    fn registry_set_get_and_clear_are_room_scoped() {
        let mut registry = ComposerDraftRegistry::new();
        let draft = NativeComposerReplyDraft {
            event_id: "$evt:example.org".into(),
            sender_id: "@alice:example.org".into(),
            body: "hello".into(),
            formatted_body: Some("<p>hello</p>".into()),
            thread_root_event_id: None,
        };
        registry.set("!room:example.org".into(), draft.clone());
        assert_eq!(registry.get("!room:example.org"), Some(&draft));
        assert!(registry.get("!other:example.org").is_none());
        assert!(registry.clear("!room:example.org"));
        assert!(!registry.clear("!room:example.org"));
        assert_eq!(
            reply_draft_readback("!room:example.org".into(), "cleared", None).status,
            "cleared"
        );
    }

    #[test]
    fn set_reply_draft_request_accepts_optional_start_thread() {
        let request: NativeComposerSetReplyDraftRequest =
            serde_json::from_value(serde_json::json!({
                "roomId": "!room:example.org",
                "eventId": "$evt:example.org",
                "startThread": true
            }))
            .unwrap();
        assert!(request.start_thread);
        assert_eq!(request.event_id, "$evt:example.org");
    }
}

mod media_pure {
    use super::*;
    use matrix_sdk::ruma::events::room::MediaSource;

    #[test]
    fn handles_are_opaque_and_revoked_with_their_timeline_item() {
        let mut registry = TimelineMediaRegistry::new(7, "live:!room:example.org");
        let handle = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/image".into()),
                Some("image/png".into()),
                Some(32),
                Some(16),
                None,
            )
            .unwrap();
        let json = serde_json::to_string(&handle).unwrap();
        assert!(!json.contains("mxc://"));
        assert!(is_timeline_media_handle(&handle.handle_id));
        assert_eq!(
            handle.handle_id.len(),
            TIMELINE_MEDIA_HANDLE_PREFIX.len() + 64
        );
        assert_eq!(registry.len(), 1);
        assert!(registry.resolve(&handle.handle_id).is_some());
        assert_eq!(registry.revoke_item("item-1"), 1);
        assert!(registry.resolve(&handle.handle_id).is_none());
    }

    #[test]
    fn reprojection_is_stable_and_retention_is_stream_bound() {
        let mut registry = TimelineMediaRegistry::new(7, "focused:!room:example.org:$event");
        let first = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/one".into()),
                Some("image/png".into()),
                None,
                None,
                None,
            )
            .unwrap();
        let updated = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/two".into()),
                Some("image/jpeg".into()),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(first.handle_id, updated.handle_id);
        assert_eq!(registry.session_generation(), 7);
        assert_eq!(registry.stream_id(), "focused:!room:example.org:$event");
        registry.retain_items(["another-item"]);
        assert!(registry.resolve(&first.handle_id).is_none());
    }
}

mod view_pure {
    use super::*;
    use matrix_sdk::ruma::events::room::message::{
        EmoteMessageEventContent, NoticeMessageEventContent, TextMessageEventContent,
    };
    use std::collections::HashMap;

    use matrix_sdk::ruma::events::room::message::MessageType;
    #[test]
    fn formatted_body_projects_distinct_html_only() {
        let rich = MessageType::Text(TextMessageEventContent::html(
            "hello",
            "<p><strong>hello</strong></p>",
        ));
        assert_eq!(
            project_formatted_body(&rich).as_deref(),
            Some("<p><strong>hello</strong></p>")
        );

        let plain = MessageType::Text(TextMessageEventContent::plain("hello"));
        assert_eq!(project_formatted_body(&plain), None);

        let same = MessageType::Notice(NoticeMessageEventContent::html("note", "note"));
        assert_eq!(project_formatted_body(&same), None);

        let emote = MessageType::Emote(EmoteMessageEventContent::html("waves", "<em>waves</em>"));
        assert_eq!(
            project_formatted_body(&emote).as_deref(),
            Some("<em>waves</em>")
        );
    }

    #[test]
    fn message_type_labels_cover_text_notice_and_emote() {
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Text(TextMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("text")
        );
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Notice(NoticeMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("notice")
        );
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Emote(EmoteMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("emote")
        );
    }

    #[test]
    fn poll_answers_project_counts_and_own_without_voter_ids() {
        let mut votes = HashMap::new();
        votes.insert(
            "a1".into(),
            vec!["@alice:example.org".into(), "@bob:example.org".into()],
        );
        votes.insert("a2".into(), vec!["@carol:example.org".into()]);

        let answers = project_poll_answers(
            [
                ("a1".into(), "Yes".into()),
                ("a2".into(), "No".into()),
                ("a3".into(), "Maybe".into()),
            ],
            &votes,
            Some("@alice:example.org"),
        );

        assert_eq!(
            answers,
            vec![
                TimelinePollAnswer {
                    id: "a1".into(),
                    text: "Yes".into(),
                    vote_count: 2,
                    own: true,
                },
                TimelinePollAnswer {
                    id: "a2".into(),
                    text: "No".into(),
                    vote_count: 1,
                    own: false,
                },
                TimelinePollAnswer {
                    id: "a3".into(),
                    text: "Maybe".into(),
                    vote_count: 0,
                    own: false,
                },
            ]
        );

        let row = TimelinePollRow {
            event: TimelineEventRowBase {
                item_id: "poll-item".into(),
                event_id: Some("$poll:example.org".into()),
                sender_id: "@alice:example.org".into(),
                sender_name: "@alice:example.org".into(),
                sender_avatar_url: None,
                origin_server_ts: 1,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: false,
                    edit: false,
                    redact: true,
                    report: false,
                    pin: true,
                    forward: false,
                    vote: true,
                    decline_call: false,
                },
            },
            question: "Lunch?".into(),
            closed: false,
            max_selections: 1,
            answers,
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("\"voteCount\":2"));
        assert!(json.contains("\"own\":true"));
        assert!(!json.contains("@bob:example.org"));
        assert!(!json.contains("@carol:example.org"));
        assert!(!json.contains("token"));
        assert!(!json.contains("ciphertext"));
    }

    #[test]
    fn reply_and_thread_summary_serialize_product_shape_without_secrets() {
        let reply = TimelineReplyPreview {
            event_id: "$parent:example.org".into(),
            sender_id: Some("@alice:example.org".into()),
            sender_name: "alice".into(),
            body: "Earlier message".into(),
        };
        let thread = TimelineThreadSummary {
            root_event_id: "$root:example.org".into(),
            reply_count: 3,
            latest_event_id: Some("$latest:example.org".into()),
        };
        let message = TimelineMessageRow {
            event: TimelineEventRowBase {
                item_id: "msg-item".into(),
                event_id: Some("$msg:example.org".into()),
                sender_id: "@bob:example.org".into(),
                sender_name: "@bob:example.org".into(),
                sender_avatar_url: None,
                origin_server_ts: 1,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: true,
                    edit: true,
                    redact: true,
                    report: false,
                    pin: true,
                    forward: true,
                    vote: false,
                    decline_call: false,
                },
            },
            body: "Reply body".into(),
            formatted_body: None,
            message_type: Some("text".into()),
            edited: false,
            reply: Some(reply),
            thread: Some(thread),
            reactions: Vec::new(),
            media: None,
            agent_card_json: None,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"eventId\":\"$parent:example.org\""));
        assert!(json.contains("\"rootEventId\":\"$root:example.org\""));
        assert!(json.contains("\"replyCount\":3"));
        assert!(json.contains("\"latestEventId\":\"$latest:example.org\""));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("access_token"));
        assert!(!json.contains("mxc://"));
    }

    #[test]
    fn snapshot_and_delta_project_pinned_event_ids_without_secrets() {
        let snapshot = TimelineViewSnapshot {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            room_id: "!room:example.org".into(),
            revision: 0,
            position: TimelineViewPosition::LiveBottom,
            pagination: TimelinePaginationState {
                backward: TimelinePageState::Available,
                forward: TimelinePageState::Available,
            },
            read_state: TimelineReadState {
                own_read_event_id: None,
                unread_anchor_event_id: None,
                is_marked_unread: false,
            },
            pinned_event_ids: vec!["$pin:example.org".into(), "$pin2:example.org".into()],
            rows: Vec::new(),
            capabilities: TimelineViewCapabilities {
                mark_read: true,
                mark_unread: true,
                paginate_backward: true,
                paginate_forward: true,
            },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"pinnedEventIds\""));
        assert!(json.contains("$pin:example.org"));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("access_token"));

        let batch = TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            stream_id: "live:!room:example.org:1".into(),
            room_id: "!room:example.org".into(),
            revision: 1,
            ops: Vec::new(),
            read_state: None,
            pagination: None,
            pinned_event_ids: Some(vec!["$pin:example.org".into()]),
        };
        let batch_json = serde_json::to_string(&batch).unwrap();
        assert!(batch_json.contains("\"pinnedEventIds\""));
        assert!(batch_json.contains("$pin:example.org"));
    }
}
