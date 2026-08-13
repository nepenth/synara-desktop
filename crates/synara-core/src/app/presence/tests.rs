//! Unit tests for P4.7 presence index.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_presence_markers(), MATRIX_PRESENCE_MARKER);
}

#[test]
fn set_and_get() {
    let mut idx = PresenceIndex::new(3);
    let snap = idx
        .set(
            "@alice:example.org",
            PresenceState::Online,
            true,
            Some(1_700_000_000_000),
            Some("coffee".into()),
        )
        .unwrap();
    assert_eq!(snap.user_id, "@alice:example.org");
    assert_eq!(snap.state, PresenceState::Online);
    assert!(snap.state.is_active());
    assert_eq!(idx.state_of("@alice:example.org"), PresenceState::Online);
    assert_eq!(
        idx.get("@alice:example.org").unwrap().status_msg.as_deref(),
        Some("coffee")
    );
}

#[test]
fn remove_and_unknown() {
    let mut idx = PresenceIndex::new(1);
    idx.set("@a:example.org", PresenceState::Offline, false, None, None)
        .unwrap();
    idx.remove("@a:example.org").unwrap();
    assert!(idx.is_empty());
    assert_eq!(idx.state_of("@a:example.org"), PresenceState::Unknown);
}

#[test]
fn active_user_ids_sorted() {
    let mut idx = PresenceIndex::new(1);
    idx.set("@bob:example.org", PresenceState::Online, true, None, None)
        .unwrap();
    idx.set(
        "@alice:example.org",
        PresenceState::Unavailable,
        false,
        None,
        None,
    )
    .unwrap();
    idx.set(
        "@carol:example.org",
        PresenceState::Offline,
        false,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        idx.active_user_ids(),
        vec![
            "@alice:example.org".to_owned(),
            "@bob:example.org".to_owned()
        ]
    );
}

#[test]
fn invalid_user_rejected() {
    let mut idx = PresenceIndex::new(1);
    let err = idx
        .set("not-a-user", PresenceState::Online, false, None, None)
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.7-invalid-user-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn status_msg_cap() {
    let mut idx = PresenceIndex::new(1);
    let long = "x".repeat(MAX_STATUS_MSG_CHARS + 1);
    let err = idx
        .set(
            "@a:example.org",
            PresenceState::Online,
            false,
            None,
            Some(long),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.7-status-msg-cap");
}

#[test]
fn timestamp_must_be_safe_for_ipc() {
    let mut idx = PresenceIndex::new(1);
    let err = idx
        .set(
            "@a:example.org",
            PresenceState::Online,
            false,
            Some(MAX_PRESENCE_TIMESTAMP_MS + 1),
            None,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.7-last-active-ts-invalid");
}

#[test]
fn user_cap_enforced() {
    let mut idx = PresenceIndex::new(1);
    for i in 0..MAX_PRESENCE_USERS {
        idx.set(
            format!("@u{i}:example.org"),
            PresenceState::Online,
            true,
            None,
            None,
        )
        .unwrap();
    }
    let err = idx
        .set(
            "@overflow:example.org",
            PresenceState::Online,
            true,
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.7-presence-user-cap");
    // Upsert existing does not hit cap
    idx.set("@u0:example.org", PresenceState::Offline, false, None, None)
        .unwrap();
    assert_eq!(idx.state_of("@u0:example.org"), PresenceState::Offline);
}

#[test]
fn retire_generation_wipes() {
    let mut idx = PresenceIndex::new(1);
    idx.set("@a:example.org", PresenceState::Online, true, None, None)
        .unwrap();
    idx.retire_generation(9);
    assert_eq!(idx.session_generation(), 9);
    assert!(idx.is_empty());
}
