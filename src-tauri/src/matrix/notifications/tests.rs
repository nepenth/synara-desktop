//! Unit tests for P7.1 notification candidate index.

use super::*;
use crate::matrix::dto::{NotificationCandidate, NotificationKind};
use crate::matrix::ipc::MatrixIpcErrorCategory;

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

fn stream_candidate(room: &str, event: &str, kind: CandidateKind) -> Candidate {
    Candidate::new(room.into(), event.into(), kind)
}

#[test]
fn stream_push_assigns_ordered_sequences() {
    let mut stream = NotificationCandidateStream::new(7);
    assert_eq!(
        stream
            .push(stream_candidate(
                "!r:example.org",
                "$one",
                CandidateKind::Message,
            ))
            .unwrap(),
        1
    );
    assert_eq!(
        stream
            .push(stream_candidate(
                "!r:example.org",
                "$two",
                CandidateKind::Mention,
            ))
            .unwrap(),
        2
    );

    let recent = stream.list_recent(10);
    assert_eq!(recent[0].event_id, "$one");
    assert_eq!(recent[1].event_id, "$two");
    assert_eq!(recent[1].sequence, 2);
}

#[test]
fn stream_push_rejects_duplicate_event_key() {
    let mut stream = NotificationCandidateStream::new(1);
    stream
        .push(stream_candidate(
            "!r:example.org",
            "$event",
            CandidateKind::Message,
        ))
        .unwrap();
    let error = stream
        .push(stream_candidate(
            "!r:example.org",
            "$event",
            CandidateKind::Invite,
        ))
        .unwrap_err();

    assert_eq!(error.diagnostic_id(), "p9.2-duplicate-candidate");
    assert_eq!(stream.len(), 1);
}

#[test]
fn stream_upsert_replaces_and_moves_to_newest() {
    let mut stream = NotificationCandidateStream::new(1);
    stream
        .push(stream_candidate(
            "!r:example.org",
            "$one",
            CandidateKind::Message,
        ))
        .unwrap();
    stream
        .push(stream_candidate(
            "!r:example.org",
            "$two",
            CandidateKind::Message,
        ))
        .unwrap();
    let sequence = stream
        .upsert(stream_candidate(
            "!r:example.org",
            "$one",
            CandidateKind::Mention,
        ))
        .unwrap();

    let recent = stream.list_recent(10);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].event_id, "$two");
    assert_eq!(recent[1].event_id, "$one");
    assert_eq!(recent[1].kind, CandidateKind::Mention);
    assert_eq!(recent[1].sequence, sequence);
    assert_eq!(sequence, 3);
}

#[test]
fn stream_marks_suppressed_without_reordering() {
    let mut stream = NotificationCandidateStream::new(1);
    stream
        .push(stream_candidate(
            "!r:example.org",
            "$one",
            CandidateKind::Invite,
        ))
        .unwrap();

    assert!(stream.mark_suppressed("!r:example.org", "$one", true));
    assert!(!stream.mark_suppressed("!r:example.org", "$missing", true));
    let candidate = stream.list_recent(1)[0];
    assert!(candidate.suppressed);
    assert_eq!(candidate.sequence, 1);
}

#[test]
fn stream_list_recent_honors_cap_and_chronology() {
    let mut stream = NotificationCandidateStream::new(1);
    for index in 0..5 {
        stream
            .push(stream_candidate(
                "!r:example.org",
                &format!("${index}"),
                CandidateKind::Other,
            ))
            .unwrap();
    }

    let recent = stream.list_recent(2);
    assert_eq!(
        recent
            .iter()
            .map(|candidate| candidate.event_id.as_str())
            .collect::<Vec<_>>(),
        ["$3", "$4"]
    );
    assert!(stream.list_recent(0).is_empty());
}

#[test]
fn stream_retention_cap_drops_oldest() {
    let mut stream = NotificationCandidateStream::new(1);
    for index in 0..=MAX_NOTIFICATION_STREAM_CANDIDATES {
        stream
            .push(stream_candidate(
                "!r:example.org",
                &format!("${index}"),
                CandidateKind::Message,
            ))
            .unwrap();
    }

    assert_eq!(stream.len(), MAX_NOTIFICATION_STREAM_CANDIDATES);
    let recent = stream.list_recent(MAX_NOTIFICATION_STREAM_CANDIDATES);
    assert_eq!(recent.first().unwrap().event_id, "$1");
    assert_eq!(
        recent.last().unwrap().sequence,
        (MAX_NOTIFICATION_STREAM_CANDIDATES + 1) as u64
    );
}

#[test]
fn stream_rejects_invalid_identifiers_without_consuming_sequence() {
    let mut stream = NotificationCandidateStream::new(1);
    let error = stream
        .push(stream_candidate(
            "not-a-room",
            "$event",
            CandidateKind::Message,
        ))
        .unwrap_err();
    assert_eq!(error.diagnostic_id(), "p9.2-invalid-room-id");

    let error = stream
        .push(stream_candidate(
            "!r:example.org",
            "not-an-event",
            CandidateKind::Message,
        ))
        .unwrap_err();
    assert_eq!(error.diagnostic_id(), "p9.2-invalid-event-id");

    assert_eq!(
        stream
            .push(stream_candidate(
                "!r:example.org",
                "$valid",
                CandidateKind::Message,
            ))
            .unwrap(),
        1
    );
}

#[test]
fn stream_retire_generation_wipes_and_resets_sequence() {
    let mut stream = NotificationCandidateStream::new(2);
    stream
        .push(stream_candidate(
            "!r:example.org",
            "$event",
            CandidateKind::Mention,
        ))
        .unwrap();

    stream.retire_generation(3);

    assert_eq!(stream.session_generation(), 3);
    assert!(stream.is_empty());
    assert_eq!(
        stream
            .push(stream_candidate(
                "!r:example.org",
                "$next",
                CandidateKind::Other,
            ))
            .unwrap(),
        1
    );
}
