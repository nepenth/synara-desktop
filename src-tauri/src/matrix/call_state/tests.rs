//! Unit tests for P10.4 call-state projection.

use super::*;
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn member(user_localpart: &str, membership: CallMembership) -> CallMember {
    CallMember {
        user_localpart: user_localpart.into(),
        membership,
        device_label: None,
    }
}

fn session(room_id: &str, call_id: &str, members: Vec<CallMember>) -> CallSessionSummary {
    CallSessionSummary {
        room_id: room_id.into(),
        call_id: call_id.into(),
        members,
        phase: CallPhase::Idle,
    }
}

fn expect_error<T>(result: Result<T, CallStateError>) -> CallStateError {
    match result {
        Ok(_) => panic!("expected call-state error"),
        Err(error) => error,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_call_state_markers(), MATRIX_CALL_STATE_MARKER);
}

#[test]
fn upsert_orders_members_and_replaces_session() {
    let mut index = CallStateIndex::new(7);
    index
        .upsert_session(session(
            "!room:example.org",
            "call-b",
            vec![
                member("zoe", CallMembership::Invite),
                member("alice", CallMembership::Join),
            ],
        ))
        .unwrap();

    let stored = index.get("!room:example.org", "call-b").unwrap();
    assert_eq!(stored.members[0].user_localpart, "alice");
    assert_eq!(stored.members[1].user_localpart, "zoe");

    let mut replacement = session("!room:example.org", "call-b", Vec::new());
    replacement.phase = CallPhase::Ended;
    let previous = index.upsert(replacement).unwrap().unwrap();
    assert_eq!(previous.phase, CallPhase::Idle);
    assert_eq!(
        index.get("!room:example.org", "call-b").unwrap().phase,
        CallPhase::Ended
    );
}

#[test]
fn update_member_inserts_and_replaces_in_order() {
    let mut index = CallStateIndex::new(1);
    index
        .upsert(session(
            "!room:example.org",
            "call",
            vec![member("bob", CallMembership::Invite)],
        ))
        .unwrap();

    index
        .update_member(
            "!room:example.org",
            "call",
            CallMember {
                user_localpart: "alice".into(),
                membership: CallMembership::Join,
                device_label: Some("phone".into()),
            },
        )
        .unwrap();
    let previous = index
        .update_member(
            "!room:example.org",
            "call",
            member("bob", CallMembership::Leave),
        )
        .unwrap()
        .unwrap();

    assert_eq!(previous.membership, CallMembership::Invite);
    let members = &index.get("!room:example.org", "call").unwrap().members;
    assert_eq!(members[0].user_localpart, "alice");
    assert_eq!(members[1].user_localpart, "bob");
    assert_eq!(members[1].membership, CallMembership::Leave);
}

#[test]
fn list_room_is_scoped_and_ordered() {
    let mut index = CallStateIndex::new(2);
    index
        .upsert(session("!a:example.org", "call-z", Vec::new()))
        .unwrap();
    index
        .upsert(session("!b:example.org", "call-a", Vec::new()))
        .unwrap();
    index
        .upsert(session("!a:example.org", "call-a", Vec::new()))
        .unwrap();

    let calls: Vec<_> = index
        .list_room("!a:example.org")
        .into_iter()
        .map(|summary| summary.call_id.as_str())
        .collect();
    assert_eq!(calls, ["call-a", "call-z"]);
}

#[test]
fn member_cap_allows_replacement_but_not_insert() {
    let mut index = CallStateIndex::new(3);
    let members = (0..MAX_CALL_MEMBERS)
        .map(|value| member(&format!("user{value:03}"), CallMembership::Join))
        .collect();
    index
        .upsert(session("!room:example.org", "call", members))
        .unwrap();

    index
        .update_member(
            "!room:example.org",
            "call",
            member("user000", CallMembership::Leave),
        )
        .unwrap();
    let error = expect_error(index.update_member(
        "!room:example.org",
        "call",
        member("overflow", CallMembership::Invite),
    ));
    assert_eq!(error.diagnostic_id(), "p10.4-member-cap");
}

#[test]
fn rejects_full_mxid_duplicate_member_and_invalid_values() {
    let mut index = CallStateIndex::new(4);
    let error = expect_error(index.upsert(session(
        "!room:example.org",
        "call",
        vec![member("@alice:example.org", CallMembership::Join)],
    )));
    assert_eq!(error.diagnostic_id(), "p10.4-invalid-user-localpart");
    assert_eq!(error.category(), MatrixIpcErrorCategory::SdkInvariant);

    let error = expect_error(index.upsert(session(
        "!room:example.org",
        "call",
        vec![
            member("alice", CallMembership::Join),
            member("alice", CallMembership::Leave),
        ],
    )));
    assert_eq!(error.diagnostic_id(), "p10.4-duplicate-member");

    let error = expect_error(index.upsert(session("invalid-room", "call", Vec::new())));
    assert_eq!(error.diagnostic_id(), "p10.4-invalid-room-id");
}

#[test]
fn unknown_session_is_safe_and_retire_wipes() {
    let mut index = CallStateIndex::new(9);
    let error = expect_error(index.update_member(
        "!room:example.org",
        "missing",
        member("alice", CallMembership::Join),
    ));
    assert_eq!(error.diagnostic_id(), "p10.4-session-not-found");
    assert!(!error.to_string().contains("missing"));

    index
        .upsert(session("!room:example.org", "call", Vec::new()))
        .unwrap();
    index.retire_generation(10);
    assert_eq!(index.session_generation(), 10);
    assert!(index.is_empty());
}
