//! Room-list presentation shared by desktop and iOS: section ordering, the
//! favorites split, room folders, per-room unread attention with space
//! rollup, and badge totals.
//!
//! Every shell used to re-derive these rules from room summaries, and the
//! copies drifted (name normalization, tie-breaks, which rooms count toward a
//! badge). This module is the single definition. Desktop receives the result
//! on the room-list snapshot; iOS calls the pure functions through UniFFI.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::dto::{Membership, NotificationMode, RoomSummary};

/// Section sort a user picked for one room-list section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum RoomListOrder {
    /// Newest latest-event timestamp first. Rooms without one sort after
    /// those with one and keep their input order, which in a snapshot is the
    /// SDK's recency order (server bump stamp).
    Recent,
    /// Display name, ignoring case and `#`; unnamed rooms sort last.
    Name,
}

/// The fields ordering needs, with no other room data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct RoomOrderInput {
    pub room_id: String,
    pub name: Option<String>,
    /// Latest-event timestamp in milliseconds; `None` when the SDK has none.
    pub last_activity_ms: Option<u64>,
    /// `m.lowpriority` rooms sort after every other room in either order.
    pub is_low_priority: bool,
}

/// Lower-case name with `#` removed and surrounding whitespace trimmed, or
/// `None` for an unnamed room. One normalization for every shell.
pub fn normalized_room_name(name: Option<&str>) -> Option<String> {
    let name = name?.replace('#', "");
    let name = name.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_lowercase())
    }
}

fn compare_names(left: &RoomOrderInput, right: &RoomOrderInput) -> Ordering {
    let left_name = normalized_room_name(left.name.as_deref());
    let right_name = normalized_room_name(right.name.as_deref());
    match (left_name, right_name) {
        (Some(l), Some(r)) => l.cmp(&r).then_with(|| left.room_id.cmp(&right.room_id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.room_id.cmp(&right.room_id),
    }
}

fn compare_recent(left: &RoomOrderInput, right: &RoomOrderInput) -> Ordering {
    match (left.last_activity_ms, right.last_activity_ms) {
        (Some(l), Some(r)) if l != r => r.cmp(&l),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), Some(_)) => compare_names(left, right),
        // Neither has a latest event (nothing displayable in the synced
        // window). The stable sort keeps the caller's order, which the SDK
        // list sorted by the server's recency stamp; names would put a room
        // that is busy with, say, reactions below idle ones.
        (None, None) => Ordering::Equal,
    }
}

/// Low-priority rooms after the rest, then the chosen order.
fn compare_priority(left: &RoomOrderInput, right: &RoomOrderInput) -> Ordering {
    left.is_low_priority.cmp(&right.is_low_priority)
}

/// Room ids in display order for one section.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn order_room_ids(rooms: Vec<RoomOrderInput>, order: RoomListOrder) -> Vec<String> {
    let mut rooms = rooms;
    match order {
        RoomListOrder::Recent => rooms.sort_by(|left, right| {
            compare_priority(left, right).then_with(|| compare_recent(left, right))
        }),
        RoomListOrder::Name => rooms.sort_by(|left, right| {
            compare_priority(left, right).then_with(|| compare_names(left, right))
        }),
    }
    rooms.into_iter().map(|room| room.room_id).collect()
}

fn order_input(room: &RoomSummary) -> RoomOrderInput {
    RoomOrderInput {
        room_id: room.room_id.clone(),
        name: room.name.clone(),
        last_activity_ms: room.last_activity_ts,
        is_low_priority: room.is_low_priority,
    }
}

/// True for a joined room tagged `m.favourite`; such rooms leave the main list.
pub fn is_favorite_room(room: &RoomSummary) -> bool {
    room.membership == Membership::Join && room.is_favorite
}

/// One room's (or space's) unread attention. A space's counts are the sum of
/// the attention rooms below it, and `from_room_ids` names those rooms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RoomUnreadAttention {
    pub room_id: String,
    pub highlight: u32,
    pub total: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub from_room_ids: Option<Vec<String>>,
}

/// Attention for one joined, non-space, unmuted room, or `None` when the room
/// asks for none. Marked-unread counts as one.
pub fn room_attention(room: &RoomSummary) -> Option<RoomUnreadAttention> {
    if room.membership != Membership::Join || room.is_space {
        return None;
    }
    if room.notification_mode == Some(NotificationMode::Mute) {
        return None;
    }
    if !(room.marked_unread || room.unread_count > 0 || room.highlight_count > 0) {
        return None;
    }
    Some(RoomUnreadAttention {
        room_id: room.room_id.clone(),
        highlight: room.highlight_count,
        total: room
            .unread_count
            .max(room.highlight_count)
            .max(u32::from(room.marked_unread)),
        from_room_ids: None,
    })
}

/// Every ancestor space of `room_id`, following `parents` without revisiting.
fn ancestors(parents: &BTreeMap<String, Vec<String>>, room_id: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<&str> = parents
        .get(room_id)
        .map(|ids| ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    while let Some(parent) = stack.pop() {
        if parent == room_id || !seen.insert(parent.to_owned()) {
            continue;
        }
        if let Some(next) = parents.get(parent) {
            stack.extend(next.iter().map(String::as_str));
        }
    }
    seen
}

/// Per-room attention plus a rollup row for every space above an attention
/// room. Rows are sorted by room id.
pub fn unread_attention_with_rollup(
    rooms: &[RoomSummary],
    parents: &BTreeMap<String, Vec<String>>,
) -> Vec<RoomUnreadAttention> {
    let mut rows: BTreeMap<String, RoomUnreadAttention> = BTreeMap::new();
    let mut spaces: BTreeMap<String, (u32, u32, BTreeSet<String>)> = BTreeMap::new();
    for attention in rooms.iter().filter_map(room_attention) {
        for space_id in ancestors(parents, &attention.room_id) {
            let entry = spaces.entry(space_id).or_default();
            entry.0 = entry.0.saturating_add(attention.highlight);
            entry.1 = entry.1.saturating_add(attention.total);
            entry.2.insert(attention.room_id.clone());
        }
        rows.insert(attention.room_id.clone(), attention);
    }
    for (space_id, (highlight, total, from)) in spaces {
        // A space that is itself an attention room keeps its own row; spaces
        // never get attention of their own, so this only guards odd data.
        rows.entry(space_id.clone())
            .or_insert_with(|| RoomUnreadAttention {
                room_id: space_id,
                highlight,
                total,
                from_room_ids: Some(from.into_iter().collect()),
            });
    }
    rows.into_values().collect()
}

/// One badge input: a room's unread total and, when it has mentions, the
/// mention count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct RoomBadgeSource {
    pub total: u64,
    pub highlight: Option<u64>,
}

/// Inputs to the app and inbox badges.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct BadgeSummaryInput {
    pub unread: Vec<RoomBadgeSource>,
    pub later_active_count: u64,
    pub invite_count: u64,
    pub agent_approval_count: u64,
}

/// App and inbox badge numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct BadgeSummary {
    /// Later items + mentions + other unreads. Invites and approvals stay off
    /// the app icon.
    pub app_badge_count: u64,
    /// Later items + invites + agent approvals.
    pub inbox_badge_count: u64,
    pub later_active_count: u64,
    pub invite_count: u64,
    pub agent_approval_count: u64,
    pub highlight_count: u64,
    pub unread_count: u64,
}

/// A room with mentions adds its mention count; any other room adds its
/// unread total. Saturates rather than overflowing.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn summarize_badges(input: BadgeSummaryInput) -> BadgeSummary {
    let mut highlight_count = 0u64;
    let mut unread_count = 0u64;
    for source in &input.unread {
        match source.highlight {
            Some(highlight) if highlight > 0 => {
                highlight_count = highlight_count.saturating_add(highlight);
            }
            _ => unread_count = unread_count.saturating_add(source.total),
        }
    }
    let later = input.later_active_count;
    BadgeSummary {
        app_badge_count: later
            .saturating_add(highlight_count)
            .saturating_add(unread_count),
        inbox_badge_count: later
            .saturating_add(input.invite_count)
            .saturating_add(input.agent_approval_count),
        later_active_count: later,
        invite_count: input.invite_count,
        agent_approval_count: input.agent_approval_count,
        highlight_count,
        unread_count,
    }
}

/// One room's inputs to the app-icon badge.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct AppIconBadgeRoom {
    pub room_id: String,
    /// Joined rooms only; invites and left rooms never count.
    pub is_joined: bool,
    pub is_space: bool,
    /// Notification mode `Mute`.
    pub is_muted: bool,
    /// Unread mentions/highlights.
    pub highlight_count: u64,
    /// Unread events that notify under the room's push rules (the SDK's
    /// unread notification count), not the plain message count.
    pub notification_count: u64,
    pub marked_unread: bool,
}

impl From<&RoomSummary> for AppIconBadgeRoom {
    fn from(room: &RoomSummary) -> Self {
        Self {
            room_id: room.room_id.clone(),
            is_joined: room.membership == Membership::Join,
            is_space: room.is_space,
            is_muted: room.notification_mode == Some(NotificationMode::Mute),
            highlight_count: u64::from(room.highlight_count),
            notification_count: u64::from(room.notification_count),
            marked_unread: room.marked_unread,
        }
    }
}

/// Inputs to the app-icon badge.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct AppIconBadgeInput {
    pub rooms: Vec<AppIconBadgeRoom>,
    /// Later reminders whose due time has arrived and that are not completed.
    pub due_reminder_count: u64,
}

/// The app-icon number and the rooms it counted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct AppIconBadgeSummary {
    pub count: u64,
    /// Rooms that need attention, in input order without duplicates. The iOS
    /// notification extension adds 1 only for a push from a room outside
    /// this set.
    pub attention_room_ids: Vec<String>,
}

/// True when a room adds 1 to the app icon: a joined, non-space, unmuted room
/// with a mention, marked unread, or unread events that notify. A room in
/// mentions-only mode with plain unread messages but no mention adds 0.
pub fn room_needs_icon_attention(room: &AppIconBadgeRoom) -> bool {
    room.is_joined
        && !room.is_space
        && !room.is_muted
        && (room.highlight_count > 0 || room.marked_unread || room.notification_count > 0)
}

/// The app-icon badge counts conversations, not messages: 1 per room that
/// needs attention ([`room_needs_icon_attention`]) plus 1 per due reminder.
/// Invites and agent approvals stay off the icon.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn summarize_app_icon_badge(input: AppIconBadgeInput) -> AppIconBadgeSummary {
    let mut seen = BTreeSet::new();
    let attention_room_ids: Vec<String> = input
        .rooms
        .iter()
        .filter(|room| room_needs_icon_attention(room))
        .filter(|room| seen.insert(room.room_id.as_str()))
        .map(|room| room.room_id.clone())
        .collect();
    let rooms = u64::try_from(attention_room_ids.len()).unwrap_or(u64::MAX);
    AppIconBadgeSummary {
        count: rooms.saturating_add(input.due_reminder_count),
        attention_room_ids,
    }
}

/// Everything the desktop room list renders from a snapshot without
/// re-deriving room rules.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RoomListPresentation {
    /// All snapshot room ids, newest activity first.
    pub recent_order: Vec<String>,
    /// All snapshot room ids, by name.
    pub name_order: Vec<String>,
    /// Joined `m.favourite` rooms, in snapshot order.
    pub favorite_room_ids: Vec<String>,
    /// Attention rows for rooms and the spaces above them.
    pub unread: Vec<RoomUnreadAttention>,
    /// Mentions and unread totals over attention rooms (not space rows).
    pub highlight_total: u64,
    pub unread_total: u64,
    /// User room folders (`in.synara.room_folders`) in display order, with
    /// the rooms each one shows (favorites excluded) and their order and
    /// unread rollup. A room in a folder's `roomIds` shows there instead of
    /// its default section.
    #[serde(default)]
    pub folders: Vec<super::folders::RoomFolderPresentation>,
}

/// Build the presentation for one snapshot.
pub fn room_list_presentation(
    rooms: &[RoomSummary],
    parents: &BTreeMap<String, Vec<String>>,
    folders: &[crate::app::account_data::NativeRoomFolder],
) -> RoomListPresentation {
    let inputs: Vec<RoomOrderInput> = rooms.iter().map(order_input).collect();
    let unread = unread_attention_with_rollup(rooms, parents);
    let summary = summarize_badges(BadgeSummaryInput {
        unread: unread
            .iter()
            .filter(|row| row.from_room_ids.is_none())
            .map(|row| RoomBadgeSource {
                total: u64::from(row.total),
                highlight: Some(u64::from(row.highlight)),
            })
            .collect(),
        ..BadgeSummaryInput::default()
    });
    RoomListPresentation {
        recent_order: order_room_ids(inputs.clone(), RoomListOrder::Recent),
        name_order: order_room_ids(inputs, RoomListOrder::Name),
        favorite_room_ids: rooms
            .iter()
            .filter(|room| is_favorite_room(room))
            .map(|room| room.room_id.clone())
            .collect(),
        unread,
        highlight_total: summary.highlight_count,
        unread_total: summary.unread_count,
        folders: super::folders::room_folders_presentation(rooms, folders),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::room_list::summary::RoomSummaryBuilder;

    fn input(id: &str, name: Option<&str>, ts: Option<u64>) -> RoomOrderInput {
        RoomOrderInput {
            room_id: id.to_owned(),
            name: name.map(str::to_owned),
            last_activity_ms: ts,
            is_low_priority: false,
        }
    }

    #[test]
    fn low_priority_rooms_sort_last_in_both_orders() {
        let mut quiet = input("!quiet", Some("Aardvark"), Some(99));
        quiet.is_low_priority = true;
        let rooms = vec![
            quiet,
            input("!busy", Some("Zulu"), Some(50)),
            input("!idle", Some("Mike"), Some(10)),
        ];
        assert_eq!(
            order_room_ids(rooms.clone(), RoomListOrder::Recent),
            ["!busy", "!idle", "!quiet"]
        );
        assert_eq!(
            order_room_ids(rooms, RoomListOrder::Name),
            ["!idle", "!busy", "!quiet"]
        );
    }

    #[test]
    fn rooms_without_a_latest_event_keep_the_sdk_recency_order() {
        // Input order is the SDK list order (server recency stamp).
        let ids = order_room_ids(
            vec![
                input("!recent-bump", Some("Zulu"), None),
                input("!dated", Some("Mike"), Some(5)),
                input("!older-bump", Some("Alpha"), None),
            ],
            RoomListOrder::Recent,
        );
        assert_eq!(ids, ["!dated", "!recent-bump", "!older-bump"]);
    }

    #[test]
    fn name_order_ignores_case_and_hash_and_puts_unnamed_last() {
        let ids = order_room_ids(
            vec![
                input("!c", Some("#zeta"), None),
                input("!x", None, None),
                input("!a", Some("Alpha"), None),
                input("!b", Some("beta"), None),
                input("!e", Some("  "), None),
            ],
            RoomListOrder::Name,
        );
        assert_eq!(ids, ["!a", "!b", "!c", "!e", "!x"]);
    }

    #[test]
    fn recent_order_breaks_ties_by_name_then_id_and_puts_missing_last() {
        let ids = order_room_ids(
            vec![
                input("!old", Some("Old"), Some(10)),
                input("!none", Some("Aaa"), None),
                input("!tie-b", Some("Bravo"), Some(30)),
                input("!tie-a", Some("Alpha"), Some(30)),
            ],
            RoomListOrder::Recent,
        );
        assert_eq!(ids, ["!tie-a", "!tie-b", "!old", "!none"]);
    }

    fn room(id: &str) -> RoomSummaryBuilder {
        RoomSummaryBuilder::new(id).name(id)
    }

    #[test]
    fn attention_skips_spaces_muted_invites_and_read_rooms() {
        let mut muted = room("!muted:x").unread(3, 0).build().unwrap();
        muted.notification_mode = Some(NotificationMode::Mute);
        let mut space = room("!space:x").unread(3, 0).build().unwrap();
        space.is_space = true;
        let invite = room("!invite:x")
            .membership(Membership::Invite)
            .unread(1, 0)
            .build()
            .unwrap();
        let read = room("!read:x").build().unwrap();
        let marked = room("!marked:x").marked_unread(true).build().unwrap();
        let mention = room("!mention:x").unread(1, 2).build().unwrap();
        let rows: Vec<_> = [muted, space, invite, read, marked, mention]
            .iter()
            .filter_map(room_attention)
            .map(|row| (row.room_id, row.highlight, row.total))
            .collect();
        assert_eq!(
            rows,
            [
                ("!marked:x".to_owned(), 0, 1),
                ("!mention:x".to_owned(), 2, 2)
            ]
        );
    }

    #[test]
    fn space_rollup_sums_descendants_once_and_survives_cycles() {
        let rooms = vec![
            room("!a:x").unread(2, 0).build().unwrap(),
            room("!b:x").unread(1, 1).build().unwrap(),
        ];
        let parents = BTreeMap::from([
            ("!a:x".to_owned(), vec!["!child-space:x".to_owned()]),
            ("!b:x".to_owned(), vec!["!top:x".to_owned()]),
            ("!child-space:x".to_owned(), vec!["!top:x".to_owned()]),
            ("!top:x".to_owned(), vec!["!child-space:x".to_owned()]),
        ]);
        let rows = unread_attention_with_rollup(&rooms, &parents);
        let top = rows.iter().find(|row| row.room_id == "!top:x").unwrap();
        assert_eq!((top.highlight, top.total), (1, 3));
        assert_eq!(
            top.from_room_ids.as_deref(),
            Some(&["!a:x".to_owned(), "!b:x".to_owned()][..])
        );
        let child = rows
            .iter()
            .find(|row| row.room_id == "!child-space:x")
            .unwrap();
        assert_eq!((child.highlight, child.total), (1, 3));
    }

    #[test]
    fn summary_counts_mentions_instead_of_their_totals() {
        let summary = summarize_badges(BadgeSummaryInput {
            unread: vec![
                RoomBadgeSource {
                    total: 5,
                    highlight: Some(2),
                },
                RoomBadgeSource {
                    total: 3,
                    highlight: Some(0),
                },
                RoomBadgeSource {
                    total: 4,
                    highlight: None,
                },
            ],
            later_active_count: 1,
            invite_count: 2,
            agent_approval_count: 3,
        });
        assert_eq!(summary.highlight_count, 2);
        assert_eq!(summary.unread_count, 7);
        assert_eq!(summary.app_badge_count, 10);
        assert_eq!(summary.inbox_badge_count, 6);
    }

    fn icon_room(summary: &RoomSummary) -> AppIconBadgeRoom {
        AppIconBadgeRoom::from(summary)
    }

    #[test]
    fn icon_attention_counts_conversations_that_notify() {
        let mut muted = room("!muted:x")
            .unread(5, 1)
            .notifications(5)
            .build()
            .unwrap();
        muted.notification_mode = Some(NotificationMode::Mute);
        let mut space = room("!space:x")
            .unread(3, 0)
            .notifications(3)
            .build()
            .unwrap();
        space.is_space = true;
        // Mentions-only room: plain messages are unread but none notifies.
        let mut mentions_only = room("!quiet:x").unread(12, 0).build().unwrap();
        mentions_only.notification_mode = Some(NotificationMode::Mentions);
        let busy = room("!busy:x")
            .unread(40, 0)
            .notifications(40)
            .build()
            .unwrap();
        let mut mention = room("!mention:x").unread(7, 1).build().unwrap();
        mention.notification_mode = Some(NotificationMode::Mentions);
        let marked = room("!marked:x").marked_unread(true).build().unwrap();
        let invite = room("!invite:x")
            .membership(Membership::Invite)
            .unread(1, 1)
            .notifications(1)
            .build()
            .unwrap();
        let dm = room("!dm:x")
            .direct(true)
            .unread(2, 0)
            .notifications(2)
            .build()
            .unwrap();
        let read = room("!read:x").build().unwrap();

        for (summary, expected) in [
            (&muted, false),
            (&space, false),
            (&mentions_only, false),
            (&busy, true),
            (&mention, true),
            (&marked, true),
            (&invite, false),
            (&dm, true),
            (&read, false),
        ] {
            assert_eq!(
                room_needs_icon_attention(&icon_room(summary)),
                expected,
                "{}",
                summary.room_id
            );
        }

        let badge = summarize_app_icon_badge(AppIconBadgeInput {
            rooms: [
                &muted,
                &space,
                &mentions_only,
                &busy,
                &mention,
                &marked,
                &invite,
                &dm,
                &read,
            ]
            .into_iter()
            .map(icon_room)
            .collect(),
            due_reminder_count: 0,
        });
        // A room with 40 notifying messages adds 1, not 40.
        assert_eq!(badge.count, 4);
        assert_eq!(
            badge.attention_room_ids,
            ["!busy:x", "!mention:x", "!marked:x", "!dm:x"]
        );
    }

    #[test]
    fn icon_badge_adds_due_reminders_and_counts_a_room_once() {
        let busy = room("!busy:x")
            .unread(40, 2)
            .notifications(40)
            .build()
            .unwrap();
        let badge = summarize_app_icon_badge(AppIconBadgeInput {
            rooms: vec![icon_room(&busy), icon_room(&busy)],
            due_reminder_count: 2,
        });
        assert_eq!(badge.count, 3);
        assert_eq!(badge.attention_room_ids, ["!busy:x"]);
        assert_eq!(
            summarize_app_icon_badge(AppIconBadgeInput::default()),
            AppIconBadgeSummary::default()
        );
    }

    #[test]
    fn notification_count_stays_out_of_desktop_json() {
        let summary = room("!busy:x")
            .unread(4, 0)
            .notifications(4)
            .build()
            .unwrap();
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("notificationCount").is_none());
        let back: RoomSummary = serde_json::from_value(json).unwrap();
        assert_eq!(back.notification_count, 0);
        assert_eq!(back.unread_count, 4);
    }

    #[test]
    fn presentation_partitions_favorites_and_totals_rooms_not_spaces() {
        let rooms = vec![
            room("!fav:x").favorite(true).unread(1, 0).build().unwrap(),
            room("!plain:x").unread(2, 1).build().unwrap(),
            room("!invite-fav:x")
                .membership(Membership::Invite)
                .favorite(true)
                .build()
                .unwrap(),
        ];
        let parents = BTreeMap::from([("!plain:x".to_owned(), vec!["!space:x".to_owned()])]);
        let presentation = room_list_presentation(&rooms, &parents, &[]);
        assert_eq!(presentation.favorite_room_ids, ["!fav:x"]);
        assert_eq!(presentation.highlight_total, 1);
        assert_eq!(presentation.unread_total, 1);
        assert_eq!(presentation.recent_order.len(), 3);
        assert!(presentation
            .unread
            .iter()
            .any(|row| row.room_id == "!space:x"));
    }
}
