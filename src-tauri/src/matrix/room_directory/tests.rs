//! Unit tests for P6.10 room directory session.

use super::*;

fn hit(room_id: &str, name: &str) -> DirectoryRoomHit {
    DirectoryRoomHit {
        room_id: room_id.into(),
        name: Some(name.into()),
        topic: None,
        canonical_alias: Some(format!("#{name}:example.org")),
        avatar_url: Some("mxc://example.org/a".into()),
        num_joined_members: 10,
        world_readable: false,
        guest_can_join: true,
        room_type: DirectoryRoomType::Room,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_room_directory_markers(),
        MATRIX_ROOM_DIRECTORY_MARKER
    );
}

#[test]
fn begin_apply_dedup_stale() {
    let mut s = RoomDirectorySession::new(1);
    let rid = s.begin("matrix", Some("matrix.org".into())).unwrap();
    assert_eq!(s.state(), DirectorySearchState::InFlight);
    s.apply_page(
        rid,
        vec![hit("!a:example.org", "a"), hit("!b:example.org", "b")],
        Some("batch1".into()),
        true,
    )
    .unwrap();
    assert_eq!(s.hits().len(), 2);
    assert_eq!(s.next_batch(), Some("batch1"));
    assert_eq!(s.prev_batch(), None);
    // Append with dedup
    s.apply_page(
        rid,
        vec![hit("!b:example.org", "b"), hit("!c:example.org", "c")],
        None,
        false,
    )
    .unwrap();
    assert_eq!(s.hits().len(), 3);
    // Stale request ignored
    s.apply_page(rid + 99, vec![hit("!z:example.org", "z")], None, true)
        .unwrap();
    assert_eq!(s.hits().len(), 3);
}

#[test]
fn cancel_and_fail() {
    let mut s = RoomDirectorySession::new(2);
    let rid = s.begin("x", None).unwrap();
    s.cancel();
    let err = s
        .apply_page(rid, vec![hit("!a:example.org", "a")], None, true)
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-cancelled");

    let rid = s.begin("y", None).unwrap();
    s.fail(rid, "p6.10-hs-error").unwrap();
    assert_eq!(s.state(), DirectorySearchState::Failed);
    assert_eq!(s.failure_diagnostic_id(), Some("p6.10-hs-error"));
}

#[test]
fn validation() {
    let mut s = RoomDirectorySession::new(3);
    let rid = s.begin("", None).unwrap();
    let err = s
        .apply_page(
            rid,
            vec![DirectoryRoomHit {
                room_id: "bad".into(),
                name: None,
                topic: None,
                canonical_alias: None,
                avatar_url: None,
                num_joined_members: 0,
                world_readable: false,
                guest_can_join: false,
                room_type: DirectoryRoomType::Room,
            }],
            None,
            true,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-invalid-room-id");

    let rid = s.begin("ok", None).unwrap();
    let err = s
        .apply_page(
            rid,
            vec![DirectoryRoomHit {
                room_id: "!r:example.org".into(),
                name: None,
                topic: None,
                canonical_alias: None,
                avatar_url: Some("data:image/png;base64,AAA".into()),
                num_joined_members: 0,
                world_readable: false,
                guest_can_join: false,
                room_type: DirectoryRoomType::Room,
            }],
            None,
            true,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-forbidden-avatar-scheme");
}

#[test]
fn retire() {
    let mut s = RoomDirectorySession::new(4);
    s.begin("q", None).unwrap();
    s.retire_generation(9);
    assert_eq!(s.session_generation(), 9);
    assert_eq!(s.state(), DirectorySearchState::Idle);
    assert!(s.hits().is_empty());
}

#[test]
fn both_pagination_tokens_are_bounded_and_projected() {
    let mut s = RoomDirectorySession::new(5);
    let request_id = s.begin("", Some("example.org".into())).unwrap();
    s.apply_page_with_batches(
        request_id,
        vec![hit("!a:example.org", "a")],
        Some("previous".into()),
        Some("next".into()),
        true,
    )
    .unwrap();
    assert_eq!(s.prev_batch(), Some("previous"));
    assert_eq!(s.next_batch(), Some("next"));

    let err = s
        .apply_page_with_batches(
            request_id,
            vec![hit("!a:example.org", "a")],
            Some(" ".into()),
            None,
            true,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-invalid-batch");
}

#[test]
fn invalid_page_metadata_does_not_partially_replace_the_current_page() {
    let mut s = RoomDirectorySession::new(6);
    let request_id = s.begin("", None).unwrap();
    s.apply_page_with_batches(
        request_id,
        vec![hit("!old:example.org", "old")],
        None,
        Some("next".into()),
        true,
    )
    .unwrap();

    let err = s
        .apply_page_with_batches(
            request_id,
            vec![hit("!new:example.org", "new")],
            Some(" ".into()),
            Some("new-next".into()),
            true,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-invalid-batch");
    assert_eq!(s.hits()[0].room_id, "!old:example.org");
    assert_eq!(s.next_batch(), Some("next"));
}

#[test]
fn hit_bounds_fail_closed() {
    let mut s = RoomDirectorySession::new(7);
    let request_id = s.begin("", None).unwrap();
    let too_long_room_id = format!("!{}:example.org", "r".repeat(MAX_ALIAS_CHARS));
    let err = s
        .apply_page(
            request_id,
            vec![DirectoryRoomHit {
                room_id: too_long_room_id,
                name: None,
                topic: None,
                canonical_alias: None,
                avatar_url: Some(format!("mxc://example.org/{}", "a".repeat(600))),
                num_joined_members: 0,
                world_readable: false,
                guest_can_join: false,
                room_type: DirectoryRoomType::Room,
            }],
            None,
            true,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-invalid-room-id");

    let err = s
        .apply_page(
            request_id,
            vec![DirectoryRoomHit {
                room_id: "!room:example.org".into(),
                name: None,
                topic: None,
                canonical_alias: None,
                avatar_url: Some(format!("mxc://example.org/{}", "a".repeat(600))),
                num_joined_members: 0,
                world_readable: false,
                guest_can_join: false,
                room_type: DirectoryRoomType::Room,
            }],
            None,
            true,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.10-forbidden-avatar-scheme");
}
