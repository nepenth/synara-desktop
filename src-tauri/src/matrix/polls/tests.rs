//! Unit tests for P5.7 poll + state/membership projection.

use super::*;
use crate::matrix::dto::{TimelineMembershipItem, TimelineStateItem};

fn start(room: &str, eid: &str) -> PollSummary {
    PollSummary {
        room_id: room.into(),
        start_event_id: eid.into(),
        sender: "@alice:example.org".into(),
        origin_server_ts: 1_700_000_000_000,
        question: "Lunch?".into(),
        answers: vec![
            PollAnswer {
                id: "yes".into(),
                label: "Yes".into(),
            },
            PollAnswer {
                id: "no".into(),
                label: "No".into(),
            },
        ],
        phase: PollPhase::Open,
        vote_counts: Default::default(),
        total_responses: 0,
        end_event_id: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_polls_markers(), MATRIX_POLLS_MARKER);
}

#[test]
fn poll_start_response_close() {
    let mut idx = PollIndex::new(1);
    idx.upsert_start(start("!r:example.org", "$p1:example.org"))
        .unwrap();
    assert_eq!(idx.open_count(), 1);
    idx.apply_response(
        "!r:example.org",
        "$r1:example.org",
        "$p1:example.org",
        "@bob:example.org",
        vec!["yes".into()],
    )
    .unwrap();
    let p = idx.get("!r:example.org", "$p1:example.org").unwrap();
    assert_eq!(p.vote_counts.get("yes"), Some(&1));
    assert_eq!(p.total_responses, 1);

    // Same user changes vote.
    idx.apply_response(
        "!r:example.org",
        "$r2:example.org",
        "$p1:example.org",
        "@bob:example.org",
        vec!["no".into()],
    )
    .unwrap();
    let p = idx.get("!r:example.org", "$p1:example.org").unwrap();
    assert_eq!(p.vote_counts.get("yes"), Some(&0));
    assert_eq!(p.vote_counts.get("no"), Some(&1));
    assert_eq!(p.total_responses, 1);

    idx.close_poll("!r:example.org", "$p1:example.org", "$end:example.org")
        .unwrap();
    assert_eq!(
        idx.get("!r:example.org", "$p1:example.org").unwrap().phase,
        PollPhase::Closed
    );
    let err = idx
        .apply_response(
            "!r:example.org",
            "$r3:example.org",
            "$p1:example.org",
            "@carol:example.org",
            vec!["yes".into()],
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.7-poll-closed");
}

#[test]
fn poll_forbidden_and_validation() {
    let mut idx = PollIndex::new(1);
    let mut s = start("!r:example.org", "$p1:example.org");
    s.question = "leak access_token=x".into();
    let err = idx.upsert_start(s).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.7-forbidden-text");

    let err = idx
        .upsert_start(start("not-room", "$p1:example.org"))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.7-invalid-room-id");
}

#[test]
fn state_and_membership_indexes() {
    let mut st = StateEventIndex::new(2);
    st.upsert(TimelineStateItem {
        item_id: "i1".into(),
        event_id: "$s1:example.org".into(),
        room_id: "!r:example.org".into(),
        sender: "@alice:example.org".into(),
        origin_server_ts: 1,
        state_key: "".into(),
        state_type: "m.room.name".into(),
        summary: Some("Room".into()),
    })
    .unwrap();
    assert_eq!(
        st.get("!r:example.org", "m.room.name", "")
            .unwrap()
            .summary
            .as_deref(),
        Some("Room")
    );

    let mut mem = MembershipEventIndex::new(2);
    mem.upsert(TimelineMembershipItem {
        item_id: "m1".into(),
        event_id: "$m1:example.org".into(),
        room_id: "!r:example.org".into(),
        sender: "@alice:example.org".into(),
        origin_server_ts: 10,
        target_user_id: "@bob:example.org".into(),
        summary: "joined".into(),
    })
    .unwrap();
    assert_eq!(mem.list_for_room("!r:example.org").len(), 1);

    st.retire_generation(9);
    mem.retire_generation(9);
    assert!(st.is_empty());
    assert!(mem.is_empty());
}

#[test]
fn poll_retire_generation() {
    let mut idx = PollIndex::new(1);
    idx.upsert_start(start("!r:example.org", "$p1:example.org"))
        .unwrap();
    idx.retire_generation(3);
    assert_eq!(idx.session_generation(), 3);
    assert!(idx.is_empty());
}
