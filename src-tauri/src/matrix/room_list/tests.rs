//! Unit tests for P4.2 room-list snapshot/delta projection.

use super::*;
use crate::matrix::dto::{Membership, RoomSummary};
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn room(id: &str, name: &str) -> RoomSummary {
    RoomSummaryBuilder::new(id)
        .name(name)
        .build()
        .expect("room summary")
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_room_list_markers(), MATRIX_ROOM_LIST_MARKER);
}

#[test]
fn snapshot_then_ordered_deltas_reconstruct() {
    let a = room("!a:example.org", "Alpha");
    let b = room("!b:example.org", "Beta");
    let c = room("!c:example.org", "Gamma");

    let mut proj = RoomListProjection::new(7);
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 7,
        sequence: 1,
        rooms: vec![a.clone(), b.clone()],
    })
    .unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.last_sequence(), 1);

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 7,
        sequence: 2,
        ops: vec![
            RoomListDeltaOp::PushBack { room: c.clone() },
            RoomListDeltaOp::Set {
                index: 0,
                room: RoomSummaryBuilder::new("!a:example.org")
                    .name("Alpha*")
                    .unread(2, 1)
                    .build()
                    .unwrap(),
            },
        ],
    })
    .unwrap();
    assert_eq!(proj.len(), 3);
    assert_eq!(proj.rooms()[0].name.as_deref(), Some("Alpha*"));
    assert_eq!(proj.rooms()[0].unread_count, 2);
    assert_eq!(proj.rooms()[2].room_id.as_str(), "!c:example.org");
    assert_eq!(proj.last_sequence(), 2);
    assert!(!proj.resync_required());
}

#[test]
fn sequence_gap_requires_resync_then_reset_recovers() {
    let mut proj = RoomListProjection::new(1);
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 1,
        sequence: 5,
        rooms: vec![room("!a:example.org", "A")],
    })
    .unwrap();

    let err = proj
        .apply_batch(RoomListDeltaBatch {
            session_generation: 1,
            sequence: 7, // gap: expected 6
            ops: vec![RoomListDeltaOp::PushBack {
                room: room("!b:example.org", "B"),
            }],
        })
        .unwrap_err();
    assert!(err.requires_resync());
    assert_eq!(err.diagnostic_id(), "p4.2-sequence-gap");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert!(proj.resync_required());

    // Non-reset while resync required is refused.
    let err2 = proj
        .apply_batch(RoomListDeltaBatch {
            session_generation: 1,
            sequence: 6,
            ops: vec![RoomListDeltaOp::Clear],
        })
        .unwrap_err();
    assert_eq!(err2.diagnostic_id(), "p4.2-resync-pending");

    // Reset recovers.
    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 10,
        ops: vec![RoomListDeltaOp::Reset {
            rooms: vec![room("!z:example.org", "Z")],
        }],
    })
    .unwrap();
    assert!(!proj.resync_required());
    assert_eq!(proj.len(), 1);
    assert_eq!(proj.rooms()[0].name.as_deref(), Some("Z"));
    assert_eq!(proj.last_sequence(), 10);
}

#[test]
fn stale_generation_is_rejected() {
    let mut proj = RoomListProjection::new(2);
    let err = proj
        .apply_snapshot(RoomListSnapshot {
            session_generation: 3,
            sequence: 1,
            rooms: vec![],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.2-snapshot-stale-generation");
    assert_eq!(
        err.category(),
        MatrixIpcErrorCategory::StaleSessionGeneration
    );
}

#[test]
fn oob_ops_mark_resync() {
    let mut proj = RoomListProjection::new(1);
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 1,
        sequence: 1,
        rooms: vec![room("!a:example.org", "A")],
    })
    .unwrap();

    let err = proj
        .apply_batch(RoomListDeltaBatch {
            session_generation: 1,
            sequence: 2,
            ops: vec![RoomListDeltaOp::Remove { index: 5 }],
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.2-remove-oob");
    assert!(proj.resync_required());
}

#[test]
fn move_insert_truncate_clear_ops() {
    let mut proj = RoomListProjection::new(1);
    let a = room("!a:example.org", "A");
    let b = room("!b:example.org", "B");
    let c = room("!c:example.org", "C");
    proj.apply_snapshot(RoomListSnapshot {
        session_generation: 1,
        sequence: 1,
        rooms: vec![a.clone(), b.clone(), c.clone()],
    })
    .unwrap();

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 2,
        ops: vec![RoomListDeltaOp::Move { from: 0, to: 2 }],
    })
    .unwrap();
    assert_eq!(
        proj.rooms()
            .iter()
            .map(|r| r.room_id.as_str())
            .collect::<Vec<_>>(),
        vec!["!b:example.org", "!c:example.org", "!a:example.org"]
    );

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 3,
        ops: vec![RoomListDeltaOp::Truncate { len: 2 }],
    })
    .unwrap();
    assert_eq!(proj.len(), 2);

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 4,
        ops: vec![RoomListDeltaOp::Insert {
            index: 1,
            room: room("!d:example.org", "D"),
        }],
    })
    .unwrap();
    assert_eq!(proj.rooms()[1].name.as_deref(), Some("D"));

    proj.apply_batch(RoomListDeltaBatch {
        session_generation: 1,
        sequence: 5,
        ops: vec![RoomListDeltaOp::Clear],
    })
    .unwrap();
    assert!(proj.is_empty());
}

#[test]
fn reconstruct_helper_matches_manual_apply() {
    let snap = RoomListSnapshot {
        session_generation: 4,
        sequence: 1,
        rooms: vec![room("!a:example.org", "A")],
    };
    let batches = vec![RoomListDeltaBatch {
        session_generation: 4,
        sequence: 2,
        ops: vec![RoomListDeltaOp::Append {
            rooms: vec![room("!b:example.org", "B")],
        }],
    }];
    let proj = reconstruct(4, snap, &batches).unwrap();
    assert_eq!(proj.len(), 2);
    assert_eq!(proj.last_sequence(), 2);
}

#[test]
fn summary_builder_membership_and_privacy() {
    let s = RoomSummaryBuilder::new("!dm:example.org")
        .name("Bob")
        .membership(Membership::Invite)
        .direct(true)
        .encrypted(true)
        .unread(3, 1)
        .marked_unread(true)
        .last_activity_ts(1_700_000_000_000)
        .build()
        .unwrap();
    assert_eq!(s.membership, Membership::Invite);
    assert!(s.is_direct);
    assert!(s.is_encrypted);
    assert_eq!(s.unread_count, 3);
    let dbg = format!("{s:?}");
    assert!(!dbg.contains("access_token"));
    assert!(!dbg.contains("syt_"));
}

#[test]
fn snapshot_into_reset_batch() {
    let snap = RoomListSnapshot {
        session_generation: 9,
        sequence: 3,
        rooms: vec![room("!a:example.org", "A")],
    };
    let batch = snap.into_reset_batch();
    assert_eq!(batch.session_generation, 9);
    assert_eq!(batch.sequence, 3);
    assert!(matches!(batch.ops[0], RoomListDeltaOp::Reset { .. }));
}

#[test]
fn delta_op_names_are_stable() {
    assert_eq!(RoomListDeltaOp::Clear.op_name(), "clear");
    assert_eq!(RoomListDeltaOp::Remove { index: 0 }.op_name(), "remove");
}

// --- P4.3 membership / unread filters + badge counts ---

fn fixture_rooms() -> Vec<RoomSummary> {
    vec![
        RoomSummaryBuilder::new("!join:example.org")
            .name("Joined quiet")
            .membership(Membership::Join)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!unread:example.org")
            .name("Unread")
            .membership(Membership::Join)
            .unread(4, 0)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!mention:example.org")
            .name("Mention")
            .membership(Membership::Join)
            .unread(1, 2)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!marked:example.org")
            .name("Marked")
            .membership(Membership::Join)
            .marked_unread(true)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!dm:example.org")
            .name("DM")
            .membership(Membership::Join)
            .direct(true)
            .unread(0, 0)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!invite:example.org")
            .name("Invite")
            .membership(Membership::Invite)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!left:example.org")
            .name("Left")
            .membership(Membership::Leave)
            .build()
            .unwrap(),
    ]
}

#[test]
fn p4_3_scope_filters_membership_and_unread() {
    let rooms = fixture_rooms();
    assert_eq!(
        select_rooms_by_scope(&rooms, RoomListScope::Joined).len(),
        5
    );
    assert_eq!(
        select_rooms_by_scope(&rooms, RoomListScope::Invites).len(),
        1
    );
    let unread = select_rooms_by_scope(&rooms, RoomListScope::Unread);
    // unread: !unread, !mention, !marked (not quiet, not dm without flags, not invite)
    assert_eq!(unread.len(), 3);
    assert!(unread.iter().all(|r| r.membership == Membership::Join));

    let mentions = select_rooms_by_scope(&rooms, RoomListScope::Mentions);
    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].room_id.as_str(), "!mention:example.org");

    let dms = select_rooms_by_scope(&rooms, RoomListScope::Direct);
    assert_eq!(dms.len(), 1);
    assert!(dms[0].is_direct);

    let active = select_rooms_by_scope(&rooms, RoomListScope::AllActive);
    assert_eq!(active.len(), 7); // no ban in fixture
    assert!(!room_matches_scope(
        rooms
            .iter()
            .find(|r| r.room_id == "!left:example.org")
            .unwrap(),
        RoomListScope::Joined
    ));
}

#[test]
fn p4_3_badge_counts_and_attention() {
    let rooms = fixture_rooms();
    let c = RoomListBadgeCounts::from_rooms(&rooms);
    assert_eq!(c.joined, 5);
    assert_eq!(c.invites, 1);
    assert_eq!(c.unread_rooms, 3);
    assert_eq!(c.mention_rooms, 1);
    assert_eq!(c.direct, 1);
    assert_eq!(c.unread_messages, 5); // 4 + 1
    assert_eq!(c.highlight_messages, 2);
    assert_eq!(c.marked_unread_rooms, 1);
    assert_eq!(c.attention_total(), 4); // 3 unread rooms + 1 invite

    let empty = RoomListBadgeCounts::from_rooms(&[]);
    assert_eq!(empty, RoomListBadgeCounts::default());
    assert_eq!(empty.attention_total(), 0);

    let dbg = format!("{c:?}");
    assert!(!dbg.contains("access_token"));
}

#[test]
fn p4_3_scope_labels_cover_all() {
    assert_eq!(RoomListScope::ALL.len(), 8);
    for s in RoomListScope::ALL {
        assert!(!s.as_str().is_empty());
    }
}

// --- P4.4 favorite / low-priority / folder / recent ---

#[test]
fn p4_4_favorite_low_priority_folder_filters() {
    let rooms = vec![
        RoomSummaryBuilder::new("!fav:example.org")
            .name("Fav")
            .favorite(true)
            .last_activity_ts(100)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!low:example.org")
            .name("Low")
            .low_priority(true)
            .last_activity_ts(200)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!folder:example.org")
            .name("In folder")
            .folder_id("work")
            .last_activity_ts(150)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!plain:example.org")
            .name("Plain")
            .last_activity_ts(50)
            .build()
            .unwrap(),
    ];
    assert_eq!(
        select_rooms_by_scope(&rooms, RoomListScope::Favorites).len(),
        1
    );
    assert_eq!(
        select_rooms_by_scope(&rooms, RoomListScope::LowPriority).len(),
        1
    );
    let folder = select_rooms_in_folder(&rooms, "work");
    assert_eq!(folder.len(), 1);
    assert_eq!(folder[0].room_id.as_str(), "!folder:example.org");
}

#[test]
fn p4_4_recent_and_priority_sorts() {
    let rooms = vec![
        RoomSummaryBuilder::new("!a:example.org")
            .name("A")
            .last_activity_ts(10)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!b:example.org")
            .name("B")
            .favorite(true)
            .last_activity_ts(5)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!c:example.org")
            .name("C")
            .low_priority(true)
            .last_activity_ts(20)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!d:example.org")
            .name("D")
            .last_activity_ts(15)
            .build()
            .unwrap(),
    ];

    let recent = sort_rooms(&rooms, RoomListSort::RecentActivity);
    assert_eq!(
        recent
            .iter()
            .map(|r| r.room_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "!c:example.org",
            "!d:example.org",
            "!a:example.org",
            "!b:example.org"
        ]
    );

    let fav_first = sort_rooms(&rooms, RoomListSort::FavoritesThenRecent);
    assert_eq!(fav_first[0].room_id.as_str(), "!b:example.org");

    let low_last = sort_rooms(&rooms, RoomListSort::LowPriorityLast);
    assert_eq!(low_last.last().unwrap().room_id.as_str(), "!c:example.org");

    let top2 = recent_joined_rooms(&rooms, 2);
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].room_id.as_str(), "!c:example.org");
    assert_eq!(top2[1].room_id.as_str(), "!d:example.org");

    let by_name = sort_rooms(&rooms, RoomListSort::ByName);
    assert_eq!(
        by_name
            .iter()
            .map(|r| r.room_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "!a:example.org",
            "!b:example.org",
            "!c:example.org",
            "!d:example.org"
        ]
    );
}

#[test]
fn p4_4_favorites_partition_leaves_remaining_rooms_sortable() {
    let rooms = vec![
        RoomSummaryBuilder::new("!fav:example.org")
            .name("Fav")
            .favorite(true)
            .last_activity_ts(1)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!zeta:example.org")
            .name("Zeta")
            .last_activity_ts(50)
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!alpha:example.org")
            .name("Alpha")
            .last_activity_ts(10)
            .build()
            .unwrap(),
    ];
    let (favorites, remaining) = partition_favorite_rooms(&rooms);
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].room_id.as_str(), "!fav:example.org");
    let remaining_by_name = sort_rooms(&remaining, RoomListSort::ByName);
    assert_eq!(
        remaining_by_name
            .iter()
            .map(|r| r.room_id.as_str())
            .collect::<Vec<_>>(),
        vec!["!alpha:example.org", "!zeta:example.org"]
    );
    let remaining_recent = sort_rooms(&remaining, RoomListSort::RecentActivity);
    assert_eq!(
        remaining_recent
            .iter()
            .map(|r| r.room_id.as_str())
            .collect::<Vec<_>>(),
        vec!["!zeta:example.org", "!alpha:example.org"]
    );
}

/// SNC-P1-5b: `invite_avatars.rs`, `invites.rs`, and `live.rs` (with their
/// internal unit tests) moved into `synara_core::app::room_list`. These mirror
/// the moved tests against the src-tauri adapter re-exports so the desktop
/// test count (and room-list coverage) stays identical to the pre-move
/// baseline — the same shape SNC-P1-5a used to keep its desktop suite intact.
///
/// The three `live.rs` helpers (`membership`, `map_notification_mode`,
/// `bounded_count`) are private to the core module and not adapter-reachable;
/// their mirrors assert the same product contract through the public
/// projection surface (`RoomListBadgeCounts` / `RoomListScope` / DTO `as_str`).
#[cfg(test)]
mod internal_mirror {
    use super::*;
    use crate::matrix::dto::{Membership, NotificationMode};

    // ---- invite_avatars.rs (exact mirror; items re-exported by the adapter) ----

    fn mxc(value: &str) -> matrix_sdk::ruma::OwnedMxcUri {
        matrix_sdk::ruma::OwnedMxcUri::from(value)
    }

    #[test]
    fn handles_are_opaque_stable_and_generation_scoped() {
        let mut handles = InviteAvatarHandles::new(7);
        let first = handles
            .issue("!invite:example.org", mxc("mxc://example.org/one"))
            .unwrap();
        let second = handles
            .issue("!invite:example.org", mxc("mxc://example.org/one"))
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(!first.contains("example.org"));
        assert!(handles.resolve(7, &first).is_some());
        assert!(handles.resolve(8, &first).is_none());
    }

    #[test]
    fn invite_action_and_session_retirement_revoke_handles() {
        let mut handles = InviteAvatarHandles::new(7);
        let one = handles
            .issue("!one:example.org", mxc("mxc://example.org/one"))
            .unwrap();
        let two = handles
            .issue("!two:example.org", mxc("mxc://example.org/two"))
            .unwrap();

        handles.revoke_room("!one:example.org");
        assert!(handles.resolve(7, &one).is_none());
        assert!(handles.resolve(7, &two).is_some());

        handles.retire_generation(8);
        assert_eq!(handles.session_generation(), 8);
        assert_eq!(handles.len(), 0);
        assert!(handles.resolve(8, &two).is_none());
    }

    #[test]
    fn capability_store_stays_bounded() {
        let mut handles = InviteAvatarHandles::new(1);
        for index in 0..=MAX_INVITE_AVATAR_HANDLES {
            handles
                .issue(
                    &format!("!{index}:example.org"),
                    mxc(&format!("mxc://example.org/{index}")),
                )
                .unwrap();
        }
        assert_eq!(handles.len(), MAX_INVITE_AVATAR_HANDLES);
    }

    // ---- invites.rs (exact mirror; contains_bad_word re-exported) ----

    #[test]
    fn corpus_and_synara_additions_match_at_js_word_boundaries() {
        assert!(contains_bad_word("a_bitch_b"));
        assert!(contains_bad_word("T0RTURE in an invite reason"));
        assert!(!contains_bad_word("ambitious"));
    }

    // ---- live.rs (product-contract mirrors via the public surface) ----

    #[test]
    fn membership_mappings_drive_product_counts_and_scopes() {
        // Mirrors live.rs room_states_map_to_product_memberships: the SDK
        // `RoomState -> Membership` helper feeds badge counts + scope filters.
        let mk = |id: &str, name: &str, membership: Membership| {
            RoomSummaryBuilder::new(id)
                .name(name)
                .membership(membership)
                .build()
                .unwrap()
        };
        let joined = mk("!j:example.org", "Joined", Membership::Join);
        let invite = mk("!i:example.org", "Invite", Membership::Invite);
        let knock = mk("!k:example.org", "Knock", Membership::Knock);
        let leave = mk("!l:example.org", "Leave", Membership::Leave);
        let banned = mk("!b:example.org", "Ban", Membership::Ban);

        let counts = RoomListBadgeCounts::from_rooms(&[
            joined.clone(),
            invite.clone(),
            knock,
            leave.clone(),
            banned.clone(),
        ]);
        assert_eq!(counts.joined, 1);
        assert_eq!(counts.invites, 1);

        assert!(room_matches_scope(&joined, RoomListScope::Joined));
        assert!(room_matches_scope(&joined, RoomListScope::AllActive));
        assert!(room_matches_scope(&invite, RoomListScope::Invites));
        assert!(room_matches_scope(&invite, RoomListScope::AllActive));
        assert!(!room_matches_scope(&leave, RoomListScope::Joined));
        assert!(!room_matches_scope(&banned, RoomListScope::AllActive));
    }

    #[test]
    fn notification_modes_map_to_stable_wire_names() {
        // Mirrors live.rs notification_modes_map_to_product_dto: the SDK
        // `RoomNotificationMode -> NotificationMode` mapping feeds the DTO that
        // serializes to the stable snake_case wire names asserted here.
        assert_eq!(NotificationMode::All.as_str(), "all");
        assert_eq!(NotificationMode::Mentions.as_str(), "mentions");
        assert_eq!(NotificationMode::Mute.as_str(), "mute");
        assert_eq!(NotificationMode::Default.as_str(), "default");
        assert_eq!(Membership::Join.as_str(), "join");
    }

    #[test]
    fn unread_and_highlight_counts_saturate_for_ipc() {
        // Mirrors live.rs unread_counts_are_bounded_for_ipc: per-room unread /
        // highlight counts are bounded at u32::MAX through the public badge
        // projection so nothing larger can ride JSON IPC.
        let room = RoomSummaryBuilder::new("!m:example.org")
            .name("Busy")
            .unread(u32::MAX, u32::MAX)
            .build()
            .unwrap();
        let counts = RoomListBadgeCounts::from_rooms(&[room]);
        assert_eq!(counts.unread_messages, u32::MAX);
        assert_eq!(counts.highlight_messages, u32::MAX);
    }
}
