//! Unit tests for P7.1 notification candidate index.

use super::*;
use crate::dto::{NotificationCandidate, NotificationKind};
use crate::transport::MatrixIpcErrorCategory;

fn candidate(
    id: &str,
    room: &str,
    event: Option<&str>,
    kind: NotificationKind,
    suppress: bool,
) -> NotificationCandidate {
    NotificationCandidate {
        candidate_id: id.into(),
        room_id: room.into(),
        event_id: event.map(Into::into),
        kind,
        title: "t".into(),
        body: "b".into(),
        route: Some("/home/room/!r:example.org".into()),
        suppress_if_focused_room: suppress,
        is_encrypted: false,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_notifications_markers(), MATRIX_NOTIFICATIONS_MARKER);
}

#[test]
fn enqueue_list_dismiss() {
    let mut idx = NotificationIndex::new(1);
    let id = idx
        .enqueue(candidate(
            "",
            "!r:example.org",
            Some("$e1"),
            NotificationKind::Message,
            false,
        ))
        .unwrap()
        .unwrap();
    assert!(id.starts_with("notif-"));
    assert_eq!(idx.list_pending().len(), 1);
    assert!(idx.dismiss(&id));
    assert!(!idx.dismiss(&id));
    assert!(idx.is_empty());
}

#[test]
fn suppress_when_focused() {
    let mut idx = NotificationIndex::new(1);
    idx.set_focused_room(Some("!r:example.org".into()));
    let r = idx
        .enqueue(candidate(
            "c1",
            "!r:example.org",
            Some("$e1"),
            NotificationKind::Message,
            true,
        ))
        .unwrap();
    assert!(r.is_none());
    // Different room still enqueued.
    let r = idx
        .enqueue(candidate(
            "c2",
            "!other:example.org",
            Some("$e2"),
            NotificationKind::Invite,
            true,
        ))
        .unwrap();
    assert_eq!(r.as_deref(), Some("c2"));
}

#[test]
fn dedup_same_event() {
    let mut idx = NotificationIndex::new(1);
    assert!(idx
        .enqueue(candidate(
            "a",
            "!r:example.org",
            Some("$e1"),
            NotificationKind::Message,
            false,
        ))
        .unwrap()
        .is_some());
    assert!(idx
        .enqueue(candidate(
            "b",
            "!r:example.org",
            Some("$e1"),
            NotificationKind::Message,
            false,
        ))
        .unwrap()
        .is_none());
    assert_eq!(idx.len(), 1);
}

#[test]
fn invalid_rejected() {
    let mut idx = NotificationIndex::new(1);
    let err = idx
        .enqueue(NotificationCandidate {
            candidate_id: "x".into(),
            room_id: "bad".into(),
            event_id: None,
            kind: NotificationKind::Message,
            title: "t".into(),
            body: "b".into(),
            route: None,
            suppress_if_focused_room: false,
            is_encrypted: true,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.1-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn cap_drops_oldest() {
    let mut idx = NotificationIndex::new(1);
    for i in 0..MAX_PENDING_CANDIDATES {
        idx.enqueue(candidate(
            &format!("c{i}"),
            "!r:example.org",
            Some(&format!("${i}")),
            NotificationKind::Message,
            false,
        ))
        .unwrap()
        .unwrap();
    }
    assert_eq!(idx.len(), MAX_PENDING_CANDIDATES);
    idx.enqueue(candidate(
        "overflow",
        "!r:example.org",
        Some("$overflow"),
        NotificationKind::Message,
        false,
    ))
    .unwrap()
    .unwrap();
    assert_eq!(idx.len(), MAX_PENDING_CANDIDATES);
    assert!(idx.get("c0").is_none());
    assert!(idx.get("overflow").is_some());
    // Evicting the oldest pending candidate must not forget that $0 already
    // notified. Re-showing it would duplicate a delivered event.
    assert!(idx.is_duplicate("!r:example.org", "$0"));
    assert!(idx
        .enqueue(candidate(
            "again",
            "!r:example.org",
            Some("$0"),
            NotificationKind::Message,
            false,
        ))
        .unwrap()
        .is_none());
}

#[test]
fn retire_generation() {
    let mut idx = NotificationIndex::new(3);
    idx.enqueue(candidate(
        "c",
        "!r:example.org",
        Some("$e"),
        NotificationKind::LaterReminder,
        false,
    ))
    .unwrap();
    idx.set_focused_room(Some("!r:example.org".into()));
    idx.retire_generation(4);
    assert_eq!(idx.session_generation(), 4);
    assert!(idx.is_empty());
    assert!(idx.focused_room().is_none());
}

#[test]
fn rejected_collision_preserves_pending_and_allows_event_retry() {
    let mut idx = NotificationIndex::new(1);
    for i in 0..MAX_PENDING_CANDIDATES {
        idx.enqueue(candidate(
            &format!("c{i}"),
            "!r:example.org",
            Some(&format!("${i}")),
            NotificationKind::Message,
            false,
        ))
        .unwrap();
    }
    assert!(idx
        .enqueue(candidate(
            "c1",
            "!r:example.org",
            Some("$retry"),
            NotificationKind::Message,
            false
        ))
        .is_err());
    assert_eq!(idx.len(), MAX_PENDING_CANDIDATES);
    assert!(idx.get("c0").is_some());
    assert!(!idx.is_duplicate("!r:example.org", "$retry"));
    assert!(idx
        .enqueue(candidate(
            "retry",
            "!r:example.org",
            Some("$retry"),
            NotificationKind::Message,
            false
        ))
        .unwrap()
        .is_some());
}

#[test]
fn recent_event_history_is_bounded_independently_of_pending() {
    let mut idx = NotificationIndex::new(1);
    for i in 0..super::index::MAX_SEEN_EVENTS + 1 {
        let id = idx
            .enqueue(candidate(
                "",
                "!r:example.org",
                Some(&format!("${i}")),
                NotificationKind::Message,
                false,
            ))
            .unwrap()
            .unwrap();
        assert!(idx.dismiss(&id));
    }
    assert!(idx.is_empty());
    assert!(!idx.is_duplicate("!r:example.org", "$0"));
    assert!(idx.is_duplicate("!r:example.org", "$1"));
    assert!(idx.is_duplicate(
        "!r:example.org",
        &format!("${}", super::index::MAX_SEEN_EVENTS)
    ));
    idx.retire_generation(2);
    assert!(!idx.is_duplicate("!r:example.org", "$1"));
    assert!(idx
        .enqueue(candidate(
            "",
            "!r:example.org",
            Some("$1"),
            NotificationKind::Message,
            false
        ))
        .unwrap()
        .is_some());
}

#[test]
fn retained_identifiers_have_byte_bounds() {
    let mut idx = NotificationIndex::new(1);
    let oversized_event = format!("${}", "x".repeat(512));
    assert!(idx
        .enqueue(candidate(
            "",
            "!r:example.org",
            Some(&oversized_event),
            NotificationKind::Message,
            false
        ))
        .is_err());
    let oversized_room = format!("!{}:example.org", "x".repeat(512));
    assert!(idx
        .enqueue(candidate(
            "",
            &oversized_room,
            Some("$e"),
            NotificationKind::Message,
            false
        ))
        .is_err());
    assert!(idx.is_empty());
}

#[test]
fn identical_candidate_resubmission_remains_a_suppressed_duplicate() {
    let mut idx = NotificationIndex::new(1);
    let item = candidate(
        "same",
        "!r:example.org",
        Some("$same"),
        NotificationKind::Message,
        false,
    );
    assert!(idx.enqueue(item.clone()).unwrap().is_some());
    assert!(idx.enqueue(item).unwrap().is_none());
    assert_eq!(idx.len(), 1);
}
