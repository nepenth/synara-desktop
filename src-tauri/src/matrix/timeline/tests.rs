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
