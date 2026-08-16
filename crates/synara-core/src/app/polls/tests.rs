//! Unit tests for P5.7 poll and state projection indexes.

use std::collections::BTreeMap;

use super::*;
use crate::transport::MatrixIpcErrorCategory;

fn poll(room_id: &str, event_id: &str, question: &str) -> PollProjection {
    PollProjection {
        poll_event_id: event_id.into(),
        room_id: room_id.into(),
        question: question.into(),
        closed: false,
        response_counts: BTreeMap::new(),
    }
}

fn state(
    room_id: &str,
    event_id: &str,
    kind: StateProjectionKind,
    summary: &str,
) -> StateProjectionRow {
    StateProjectionRow {
        room_id: room_id.into(),
        event_id: event_id.into(),
        kind,
        target_user_localpart: None,
        summary: summary.into(),
    }
}

#[test]
fn marker_and_empty_indexes() {
    assert_eq!(matrix_polls_markers(), MATRIX_POLLS_MARKER);
    assert!(PollIndex::new(1).is_empty());
    assert!(StateProjectionIndex::new(1).is_empty());
}

#[test]
fn poll_insert_replace_and_remove() {
    let mut index = PollIndex::new(3);
    assert!(index
        .upsert(poll("!room:example.org", "$poll", "First question"))
        .unwrap()
        .is_none());

    let mut replacement = poll("!room:example.org", "$poll", "Updated question");
    replacement.closed = true;
    replacement.response_counts.insert("yes".into(), 4);
    let previous = index.upsert(replacement).unwrap().unwrap();
    assert_eq!(previous.question, "First question");

    let current = index.get("!room:example.org", "$poll").unwrap();
    assert_eq!(current.question, "Updated question");
    assert!(current.closed);
    assert_eq!(current.response_counts.get("yes"), Some(&4));
    assert_eq!(
        index
            .remove("!room:example.org", "$poll")
            .unwrap()
            .poll_event_id,
        "$poll"
    );
    assert!(index.is_empty());
}

#[test]
fn poll_iteration_is_stable_and_rooms_are_isolated() {
    let mut index = PollIndex::new(1);
    index.upsert(poll("!a:example.org", "$z", "Last")).unwrap();
    index
        .upsert(poll("!b:example.org", "$middle", "Other room"))
        .unwrap();
    index.upsert(poll("!a:example.org", "$a", "First")).unwrap();

    let room_a: Vec<_> = index
        .list_room("!a:example.org")
        .into_iter()
        .map(|row| row.poll_event_id.as_str())
        .collect();
    assert_eq!(room_a, ["$a", "$z"]);
    assert_eq!(index.list_room("!b:example.org").len(), 1);
    index.clear_room("!a:example.org");
    assert!(index.list_room("!a:example.org").is_empty());
    assert_eq!(index.list_room("!b:example.org").len(), 1);
}

#[test]
fn state_insert_replace_remove_and_localpart_validation() {
    let mut index = StateProjectionIndex::new(2);
    let mut joined = state(
        "!room:example.org",
        "$member",
        StateProjectionKind::MemberJoin,
        "Alice joined",
    );
    joined.target_user_localpart = Some("alice".into());
    index.upsert(joined).unwrap();

    let left = state(
        "!room:example.org",
        "$member",
        StateProjectionKind::MemberLeave,
        "Alice left",
    );
    let previous = index.upsert(left).unwrap().unwrap();
    assert_eq!(previous.kind, StateProjectionKind::MemberJoin);
    assert_eq!(
        index.get("!room:example.org", "$member").unwrap().kind,
        StateProjectionKind::MemberLeave
    );
    assert!(index.remove("!room:example.org", "$member").is_some());

    let mut invalid = state(
        "!room:example.org",
        "$bad",
        StateProjectionKind::MemberBan,
        "Banned",
    );
    invalid.target_user_localpart = Some("@alice:example.org".into());
    let error = index.upsert(invalid).err().unwrap();
    assert_eq!(error.diagnostic_id(), "p5.7-invalid-user-localpart");
    assert_eq!(error.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn state_iteration_is_stable_and_rooms_are_isolated() {
    let mut index = StateProjectionIndex::new(1);
    index
        .upsert(state(
            "!a:example.org",
            "$z",
            StateProjectionKind::Topic,
            "Topic Z",
        ))
        .unwrap();
    index
        .upsert(state(
            "!b:example.org",
            "$b",
            StateProjectionKind::Other,
            "Other",
        ))
        .unwrap();
    index
        .upsert(state(
            "!a:example.org",
            "$a",
            StateProjectionKind::Name,
            "Room A",
        ))
        .unwrap();

    let room_a: Vec<_> = index
        .list_room("!a:example.org")
        .into_iter()
        .map(|row| row.event_id.as_str())
        .collect();
    assert_eq!(room_a, ["$a", "$z"]);
    assert_eq!(index.list_room("!b:example.org").len(), 1);
}

#[test]
fn retire_generation_wipes_both_indexes() {
    let mut polls = PollIndex::new(4);
    polls
        .upsert(poll("!room:example.org", "$poll", "Question"))
        .unwrap();
    polls.retire_generation(5);
    assert_eq!(polls.session_generation(), 5);
    assert!(polls.is_empty());

    let mut state = StateProjectionIndex::new(4);
    state
        .upsert(self::state(
            "!room:example.org",
            "$name",
            StateProjectionKind::Name,
            "New name",
        ))
        .unwrap();
    state.retire_generation(5);
    assert_eq!(state.session_generation(), 5);
    assert!(state.is_empty());
}

#[test]
fn invalid_poll_identity_is_rejected() {
    let mut index = PollIndex::new(1);
    let error = index
        .upsert(poll("not-a-room", "$poll", "Question"))
        .err()
        .unwrap();
    assert_eq!(error.diagnostic_id(), "p5.7-invalid-room-id");
}
