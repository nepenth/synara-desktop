//! Unit tests for P5.5 unread / open-position policy.

use super::*;
use crate::dto::{Receipt, ReceiptType};

fn room_state(room: &str) -> RoomReadState {
    RoomReadState::new(room).unwrap()
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_unread_markers(), MATRIX_UNREAD_MARKER);
}

#[test]
fn unread_signal_opens_context_at_fully_read() {
    let mut store = UnreadPositionStore::new(1);
    let mut s = room_state("!r:example.org");
    s.fully_read_event_id = Some("$mark:example.org".into());
    s.notification_count = 3;
    s.has_unread_notification = true;
    store.upsert(s).unwrap();

    let policy = store.decide_open("!r:example.org", None).unwrap();
    match policy {
        OpenPositionPolicy::UnreadContext {
            marker_event_id,
            source,
        } => {
            assert_eq!(marker_event_id, "$mark:example.org");
            assert_eq!(source, FrontierSource::FullyRead);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(store.should_show_jump_to_unread("!r:example.org").unwrap());
}

#[test]
fn fully_read_prefers_live_or_viewport() {
    let mut store = UnreadPositionStore::new(1);
    let mut s = room_state("!r:example.org");
    s.fully_read_event_id = Some("$mark:example.org".into());
    store.upsert(s).unwrap();
    assert_eq!(
        store
            .decide_open("!r:example.org", None)
            .unwrap()
            .as_kind_str(),
        "live_bottom"
    );

    store
        .set_fresh_local_viewport("!r:example.org", true)
        .unwrap();
    assert_eq!(
        store
            .decide_open("!r:example.org", None)
            .unwrap()
            .as_kind_str(),
        "restore_local_viewport"
    );
}

#[test]
fn explicit_event_wins_over_unread() {
    let mut store = UnreadPositionStore::new(1);
    let mut s = room_state("!r:example.org");
    s.notification_count = 1;
    s.has_unread_notification = true;
    s.fully_read_event_id = Some("$mark:example.org".into());
    store.upsert(s).unwrap();
    let policy = store
        .decide_open("!r:example.org", Some("$route:example.org"))
        .unwrap();
    match policy {
        OpenPositionPolicy::ExplicitEvent { event_id } => {
            assert_eq!(event_id, "$route:example.org");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn apply_receipts_update_frontier() {
    let mut store = UnreadPositionStore::new(2);
    store
        .apply_receipt(Receipt {
            room_id: "!r:example.org".into(),
            event_id: "$fr:example.org".into(),
            user_id: "@me:example.org".into(),
            receipt_type: ReceiptType::FullyRead,
            ts: Some(1),
            thread_id: None,
        })
        .unwrap();
    store
        .apply_receipt(Receipt {
            room_id: "!r:example.org".into(),
            event_id: "$priv:example.org".into(),
            user_id: "@me:example.org".into(),
            receipt_type: ReceiptType::ReadPrivate,
            ts: Some(2),
            thread_id: None,
        })
        .unwrap();
    let st = store.get("!r:example.org").unwrap();
    let (id, src) = st.effective_frontier();
    assert_eq!(id, Some("$fr:example.org"));
    assert_eq!(src, FrontierSource::FullyRead);
}

#[test]
fn jump_unread_when_marker_differs_from_live() {
    let mut store = UnreadPositionStore::new(1);
    let mut s = room_state("!r:example.org");
    s.fully_read_event_id = Some("$old:example.org".into());
    s.live_bottom_event_id = Some("$new:example.org".into());
    s.notification_count = 2;
    s.has_unread_notification = true;
    store.upsert(s).unwrap();
    assert!(store.should_show_jump_to_unread("!r:example.org").unwrap());

    store
        .set_live_bottom("!r:example.org", Some("$old:example.org".into()))
        .unwrap();
    // Marker equals live bottom — still may show if unread count > 0? policy: false when equal
    assert!(!store.should_show_jump_to_unread("!r:example.org").unwrap());
}

#[test]
fn validation_and_retire() {
    let mut store = UnreadPositionStore::new(1);
    let err = RoomReadState::new("bad").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.5-invalid-room-id");
    store.upsert(room_state("!r:example.org")).unwrap();
    let err = store
        .decide_open("!r:example.org", Some("not-event"))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.5-invalid-event-id");
    store.retire_generation(9);
    assert!(store.is_empty());
    assert_eq!(store.session_generation(), 9);
}
