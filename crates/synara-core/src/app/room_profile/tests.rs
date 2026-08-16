//! Unit tests for P6.5 room profile index.

use super::*;

fn sample_profile(room_id: &str) -> RoomProfile {
    RoomProfile {
        room_id: room_id.into(),
        name: Some("General".into()),
        topic: Some("Hello".into()),
        avatar_url: Some("mxc://example.org/abc".into()),
        canonical_alias: Some("#general:example.org".into()),
        alt_aliases: vec!["#gen:example.org".into()],
        join_rule: Some(JoinRule::Invite),
        history_visibility: Some(HistoryVisibility::Shared),
        directory_visibility: Some(DirectoryVisibility::Private),
        predecessor_room_id: None,
        successor_room_id: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_room_profile_markers(), MATRIX_ROOM_PROFILE_MARKER);
}

#[test]
fn upsert_get_and_alias_lookup() {
    let mut idx = RoomProfileIndex::new(1);
    idx.upsert(sample_profile("!r:example.org")).unwrap();
    assert_eq!(idx.len(), 1);
    assert_eq!(
        idx.get("!r:example.org").unwrap().name.as_deref(),
        Some("General")
    );
    assert_eq!(
        idx.room_id_for_alias("#general:example.org"),
        Some("!r:example.org")
    );
    assert_eq!(
        idx.room_id_for_alias("#gen:example.org"),
        Some("!r:example.org")
    );
}

#[test]
fn patch_name_topic_join_history_directory() {
    let mut idx = RoomProfileIndex::new(2);
    idx.upsert(sample_profile("!r:example.org")).unwrap();
    idx.set_name("!r:example.org", Some("Renamed".into()))
        .unwrap();
    idx.set_topic("!r:example.org", None).unwrap();
    idx.set_join_rule("!r:example.org", JoinRule::Public)
        .unwrap();
    idx.set_history_visibility("!r:example.org", HistoryVisibility::WorldReadable)
        .unwrap();
    idx.set_directory_visibility("!r:example.org", DirectoryVisibility::Public)
        .unwrap();
    let p = idx.get("!r:example.org").unwrap();
    assert_eq!(p.name.as_deref(), Some("Renamed"));
    assert!(p.topic.is_none());
    assert_eq!(p.join_rule, Some(JoinRule::Public));
    assert_eq!(p.history_visibility, Some(HistoryVisibility::WorldReadable));
    assert_eq!(p.directory_visibility, Some(DirectoryVisibility::Public));
}

#[test]
fn set_aliases_and_conflict() {
    let mut idx = RoomProfileIndex::new(3);
    idx.upsert(sample_profile("!a:example.org")).unwrap();
    idx.upsert(RoomProfile {
        room_id: "!b:example.org".into(),
        name: None,
        topic: None,
        avatar_url: None,
        canonical_alias: None,
        alt_aliases: vec![],
        join_rule: None,
        history_visibility: None,
        directory_visibility: None,
        predecessor_room_id: None,
        successor_room_id: None,
    })
    .unwrap();

    let err = idx
        .set_aliases(
            "!b:example.org",
            Some("#general:example.org".into()),
            vec![],
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.5-alias-conflict");

    idx.set_aliases(
        "!a:example.org",
        Some("#new:example.org".into()),
        vec!["#alt:example.org".into()],
    )
    .unwrap();
    assert!(idx.room_id_for_alias("#general:example.org").is_none());
    assert_eq!(
        idx.room_id_for_alias("#new:example.org"),
        Some("!a:example.org")
    );
    assert_eq!(
        idx.room_id_for_alias("#alt:example.org"),
        Some("!a:example.org")
    );
}

#[test]
fn upgrade_chain_and_self_links_forbidden() {
    let mut idx = RoomProfileIndex::new(4);
    for id in ["!v1:example.org", "!v2:example.org", "!v3:example.org"] {
        idx.upsert(RoomProfile {
            room_id: id.into(),
            name: None,
            topic: None,
            avatar_url: None,
            canonical_alias: None,
            alt_aliases: vec![],
            join_rule: None,
            history_visibility: None,
            directory_visibility: None,
            predecessor_room_id: None,
            successor_room_id: None,
        })
        .unwrap();
    }
    idx.set_successor("!v1:example.org", Some("!v2:example.org".into()))
        .unwrap();
    idx.set_successor("!v2:example.org", Some("!v3:example.org".into()))
        .unwrap();
    idx.set_predecessor("!v2:example.org", Some("!v1:example.org".into()))
        .unwrap();
    assert_eq!(
        idx.upgrade_chain("!v1:example.org", 8),
        vec!["!v2:example.org".to_string(), "!v3:example.org".to_string()]
    );
    let err = idx
        .set_successor("!v1:example.org", Some("!v1:example.org".into()))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.5-self-successor");
}

#[test]
fn forbids_data_avatar_and_bad_ids() {
    let mut idx = RoomProfileIndex::new(5);
    let err = idx
        .upsert(RoomProfile {
            room_id: "bad".into(),
            name: None,
            topic: None,
            avatar_url: None,
            canonical_alias: None,
            alt_aliases: vec![],
            join_rule: None,
            history_visibility: None,
            directory_visibility: None,
            predecessor_room_id: None,
            successor_room_id: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.5-invalid-room-id");

    let err = idx
        .upsert(RoomProfile {
            room_id: "!r:example.org".into(),
            name: None,
            topic: None,
            avatar_url: Some("data:image/png;base64,AAAA".into()),
            canonical_alias: None,
            alt_aliases: vec![],
            join_rule: None,
            history_visibility: None,
            directory_visibility: None,
            predecessor_room_id: None,
            successor_room_id: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.5-forbidden-avatar-scheme");
}

#[test]
fn remove_and_retire() {
    let mut idx = RoomProfileIndex::new(6);
    idx.upsert(sample_profile("!r:example.org")).unwrap();
    assert!(idx.remove("!r:example.org"));
    assert!(idx.room_id_for_alias("#general:example.org").is_none());
    idx.upsert(sample_profile("!r:example.org")).unwrap();
    idx.retire_generation(99);
    assert!(idx.is_empty());
    assert_eq!(idx.session_generation(), 99);
}

#[test]
fn join_rule_parse_roundtrip() {
    for rule in JoinRule::ALL {
        assert_eq!(JoinRule::parse(rule.as_str()), Some(*rule));
    }
    for vis in HistoryVisibility::ALL {
        assert_eq!(HistoryVisibility::parse(vis.as_str()), Some(*vis));
    }
    assert_eq!(
        DirectoryVisibility::parse("public"),
        Some(DirectoryVisibility::Public)
    );
}
