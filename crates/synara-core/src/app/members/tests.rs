//! Unit tests for P4.6 member index.

use super::*;
use crate::dto::{Membership, RoomMember};
use crate::transport::MatrixIpcErrorCategory;

fn member(
    room: &str,
    user: &str,
    membership: Membership,
    power: i32,
    name: Option<&str>,
) -> RoomMember {
    RoomMember {
        room_id: room.into(),
        user_id: user.into(),
        display_name: name.map(Into::into),
        avatar_url: None,
        membership,
        power_level: power,
        is_direct_target: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_members_markers(), MATRIX_MEMBERS_MARKER);
}

#[test]
fn upsert_list_power_order() {
    let mut idx = MemberIndex::new(1);
    idx.upsert(member(
        "!r:example.org",
        "@bob:example.org",
        Membership::Join,
        0,
        Some("Bob"),
    ))
    .unwrap();
    idx.upsert(member(
        "!r:example.org",
        "@alice:example.org",
        Membership::Join,
        100,
        Some("Alice"),
    ))
    .unwrap();
    idx.upsert(member(
        "!r:example.org",
        "@carol:example.org",
        Membership::Invite,
        0,
        None,
    ))
    .unwrap();
    let joined = idx.list_joined("!r:example.org");
    assert_eq!(joined.len(), 2);
    assert_eq!(joined[0].user_id, "@alice:example.org");
    assert_eq!(joined[1].user_id, "@bob:example.org");
    assert_eq!(
        idx.highest_power("!r:example.org").unwrap().user_id,
        "@alice:example.org"
    );
    assert_eq!(
        idx.list_room("!r:example.org", Some(Membership::Invite))
            .len(),
        1
    );
}

#[test]
fn upsert_overwrites() {
    let mut idx = MemberIndex::new(1);
    idx.upsert(member(
        "!r:example.org",
        "@u:example.org",
        Membership::Join,
        0,
        None,
    ))
    .unwrap();
    idx.upsert(member(
        "!r:example.org",
        "@u:example.org",
        Membership::Leave,
        50,
        Some("U"),
    ))
    .unwrap();
    let m = idx.get("!r:example.org", "@u:example.org").unwrap();
    assert_eq!(m.membership, Membership::Leave);
    assert_eq!(m.power_level, 50);
    assert_eq!(m.display_name.as_deref(), Some("U"));
    assert_eq!(idx.member_count("!r:example.org"), 1);
}

#[test]
fn remove_clear_retire() {
    let mut idx = MemberIndex::new(2);
    idx.upsert(member(
        "!a:example.org",
        "@u:example.org",
        Membership::Join,
        0,
        None,
    ))
    .unwrap();
    idx.upsert(member(
        "!b:example.org",
        "@u:example.org",
        Membership::Join,
        0,
        None,
    ))
    .unwrap();
    assert!(idx.remove("!a:example.org", "@u:example.org"));
    assert!(!idx.remove("!a:example.org", "@u:example.org"));
    idx.clear_room("!b:example.org");
    assert!(idx.is_empty());
    idx.upsert(member(
        "!c:example.org",
        "@u:example.org",
        Membership::Join,
        0,
        None,
    ))
    .unwrap();
    idx.retire_generation(9);
    assert_eq!(idx.session_generation(), 9);
    assert!(idx.is_empty());
}

#[test]
fn invalid_ids_rejected() {
    let mut idx = MemberIndex::new(1);
    let err = idx
        .upsert(member("bad", "@u:example.org", Membership::Join, 0, None))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.6-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    let err = idx
        .upsert(member("!r:example.org", "bad", Membership::Join, 0, None))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.6-invalid-user-id");
}

#[test]
fn batch_and_cap() {
    let mut idx = MemberIndex::new(1);
    let n = idx
        .upsert_batch(vec![
            member(
                "!r:example.org",
                "@a:example.org",
                Membership::Join,
                0,
                None,
            ),
            member(
                "!r:example.org",
                "@b:example.org",
                Membership::Join,
                0,
                None,
            ),
        ])
        .unwrap();
    assert_eq!(n, 2);

    let mut idx = MemberIndex::new(1);
    for i in 0..MAX_MEMBERS_PER_ROOM {
        idx.upsert(member(
            "!r:example.org",
            &format!("@u{i}:example.org"),
            Membership::Join,
            0,
            None,
        ))
        .unwrap();
    }
    let err = idx
        .upsert(member(
            "!r:example.org",
            "@overflow:example.org",
            Membership::Join,
            0,
            None,
        ))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.6-member-cap");
    // Overwrite at cap still ok.
    idx.upsert(member(
        "!r:example.org",
        "@u0:example.org",
        Membership::Ban,
        99,
        Some("zero"),
    ))
    .unwrap();
    assert_eq!(
        idx.get("!r:example.org", "@u0:example.org")
            .unwrap()
            .membership,
        Membership::Ban
    );
}
