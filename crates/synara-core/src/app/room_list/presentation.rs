//! Room-list presentation shared by desktop and iOS: section ordering, the
//! favorites split, per-room unread attention with space rollup, and badge
//! totals.
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
    /// Newest latest-event timestamp first; rooms without one sort last, then by name.
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
        _ => compare_names(left, right),
    }
}

/// Room ids in display order for one section.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn order_room_ids(rooms: Vec<RoomOrderInput>, order: RoomListOrder) -> Vec<String> {
    let mut rooms = rooms;
    match order {
        RoomListOrder::Recent => rooms.sort_by(compare_recent),
        RoomListOrder::Name => rooms.sort_by(compare_names),
    }
    rooms.into_iter().map(|room| room.room_id).collect()
}

fn order_input(room: &RoomSummary) -> RoomOrderInput {
    RoomOrderInput {
        room_id: room.room_id.clone(),
        name: room.name.clone(),
        last_activity_ms: room.last_activity_ts,
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
}

/// Build the presentation for one snapshot.
pub fn room_list_presentation(
    rooms: &[RoomSummary],
    parents: &BTreeMap<String, Vec<String>>,
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
        }
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
        let presentation = room_list_presentation(&rooms, &parents);
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
