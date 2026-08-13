//! Unit tests for P6.3 typing index.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_typing_markers(), MATRIX_TYPING_MARKER);
}

#[test]
fn nonempty_snapshots_are_sorted_and_omit_empty_rooms() {
    let mut idx = TypingIndex::new(2);
    idx.set_users("!b:example.org", ["@bob:example.org"])
        .unwrap();
    idx.set_users("!a:example.org", ["@alice:example.org"])
        .unwrap();
    idx.clear_room("!b:example.org").unwrap();
    let snaps = idx.nonempty_snapshots();
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].room_id, "!a:example.org");
}

#[test]
fn set_users_and_snapshot() {
    let mut idx = TypingIndex::new(3);
    let snap = idx
        .set_users("!r:example.org", ["@alice:example.org", "@bob:example.org"])
        .unwrap();
    assert_eq!(snap.room_id, "!r:example.org");
    assert_eq!(snap.user_ids.len(), 2);
    assert!(idx.is_typing("!r:example.org", "@alice:example.org"));
    assert!(!idx.is_typing("!r:example.org", "@carol:example.org"));
}

#[test]
fn add_remove_user() {
    let mut idx = TypingIndex::new(1);
    idx.add_user("!r:example.org", "@a:example.org").unwrap();
    idx.add_user("!r:example.org", "@b:example.org").unwrap();
    assert_eq!(idx.snapshot("!r:example.org").user_ids.len(), 2);
    idx.remove_user("!r:example.org", "@a:example.org").unwrap();
    assert!(!idx.is_typing("!r:example.org", "@a:example.org"));
    idx.remove_user("!r:example.org", "@b:example.org").unwrap();
    assert!(idx.snapshot("!r:example.org").user_ids.is_empty());
    assert_eq!(idx.room_count(), 0);
}

#[test]
fn empty_set_clears_room() {
    let mut idx = TypingIndex::new(1);
    idx.set_users("!r:example.org", ["@a:example.org"]).unwrap();
    idx.set_users("!r:example.org", Vec::<String>::new())
        .unwrap();
    assert!(idx.is_empty());
}

#[test]
fn invalid_ids_rejected() {
    let mut idx = TypingIndex::new(1);
    let err = idx.set_users("bad", ["@a:example.org"]).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.3-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    let err = idx.add_user("!r:example.org", "not-a-user").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.3-invalid-user-id");
}

#[test]
fn user_cap_enforced() {
    let mut idx = TypingIndex::new(1);
    let users: Vec<String> = (0..=MAX_TYPING_USERS_PER_ROOM)
        .map(|i| format!("@u{i}:example.org"))
        .collect();
    let err = idx.set_users("!r:example.org", users).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.3-typing-user-cap");
}

#[test]
fn retire_generation_wipes() {
    let mut idx = TypingIndex::new(1);
    idx.add_user("!r:example.org", "@a:example.org").unwrap();
    idx.retire_generation(7);
    assert_eq!(idx.session_generation(), 7);
    assert!(idx.is_empty());
}
