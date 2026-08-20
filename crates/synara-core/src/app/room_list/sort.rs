//! Room-list ordering helpers (P4.4 recent + favorite priority).
//!
//! Pure sorts over [`RoomSummary`] — no SDK types.

use crate::dto::RoomSummary;

/// Sort keys for product room lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomListSort {
    /// Case-insensitive display name; missing name sorts last; stable by room_id.
    ByName,
    /// Newest `last_activity_ts` first; missing ts sorts last; stable by room_id.
    RecentActivity,
    /// Favorites first, then recent activity among the rest.
    FavoritesThenRecent,
    /// Low-priority rooms last, then recent activity.
    LowPriorityLast,
}

impl RoomListSort {
    pub const ALL: &'static [RoomListSort] = &[
        Self::ByName,
        Self::RecentActivity,
        Self::FavoritesThenRecent,
        Self::LowPriorityLast,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ByName => "by_name",
            Self::RecentActivity => "recent_activity",
            Self::FavoritesThenRecent => "favorites_then_recent",
            Self::LowPriorityLast => "low_priority_last",
        }
    }
}

/// Return a new ordered vector of room clones.
pub fn sort_rooms(rooms: &[RoomSummary], sort: RoomListSort) -> Vec<RoomSummary> {
    let mut out = rooms.to_vec();
    sort_rooms_in_place(&mut out, sort);
    out
}

/// Sort `rooms` in place.
pub fn sort_rooms_in_place(rooms: &mut [RoomSummary], sort: RoomListSort) {
    rooms.sort_by(|a, b| {
        use std::cmp::Ordering;
        let by_name =
            |x: &RoomSummary, y: &RoomSummary| match (normalized_name(x), normalized_name(y)) {
                (Some(nx), Some(ny)) => nx.cmp(&ny),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => x.room_id.cmp(&y.room_id),
            };
        let by_recent =
            |x: &RoomSummary, y: &RoomSummary| match (x.last_activity_ts, y.last_activity_ts) {
                (Some(tx), Some(ty)) => ty.cmp(&tx), // newer first
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => x.room_id.cmp(&y.room_id),
            };
        match sort {
            RoomListSort::ByName => by_name(a, b).then_with(|| a.room_id.cmp(&b.room_id)),
            RoomListSort::RecentActivity => by_recent(a, b).then_with(|| a.room_id.cmp(&b.room_id)),
            RoomListSort::FavoritesThenRecent => match (a.is_favorite, b.is_favorite) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => by_recent(a, b).then_with(|| a.room_id.cmp(&b.room_id)),
            },
            RoomListSort::LowPriorityLast => match (a.is_low_priority, b.is_low_priority) {
                (false, true) => Ordering::Less,
                (true, false) => Ordering::Greater,
                _ => by_recent(a, b).then_with(|| a.room_id.cmp(&b.room_id)),
            },
        }
    });
}

/// Top-N recent joined rooms by activity (for "recent" rail).
pub fn recent_joined_rooms(rooms: &[RoomSummary], limit: usize) -> Vec<RoomSummary> {
    let mut joined: Vec<RoomSummary> = rooms
        .iter()
        .filter(|r| r.membership == crate::dto::Membership::Join)
        .cloned()
        .collect();
    sort_rooms_in_place(&mut joined, RoomListSort::RecentActivity);
    if joined.len() > limit {
        joined.truncate(limit);
    }
    joined
}

fn normalized_name(room: &RoomSummary) -> Option<String> {
    let name = room.name.as_deref()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.replace('#', "").to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::room_list::summary::RoomSummaryBuilder;

    fn named(id: &str, name: &str, ts: u64) -> RoomSummary {
        RoomSummaryBuilder::new(id)
            .name(name)
            .last_activity_ts(ts)
            .build()
            .unwrap()
    }

    #[test]
    fn by_name_is_case_insensitive_and_ignores_hash() {
        let rooms = vec![
            named("!c:example.org", "#zeta", 1),
            named("!a:example.org", "Alpha", 9),
            named("!b:example.org", "beta", 5),
        ];
        let sorted = sort_rooms(&rooms, RoomListSort::ByName);
        assert_eq!(
            sorted
                .iter()
                .map(|r| r.room_id.as_str())
                .collect::<Vec<_>>(),
            vec!["!a:example.org", "!b:example.org", "!c:example.org"]
        );
    }

    #[test]
    fn recent_activity_orders_newest_first() {
        let rooms = vec![
            named("!old:example.org", "Old", 10),
            named("!new:example.org", "New", 30),
            named("!mid:example.org", "Mid", 20),
        ];
        let sorted = sort_rooms(&rooms, RoomListSort::RecentActivity);
        assert_eq!(
            sorted
                .iter()
                .map(|r| r.room_id.as_str())
                .collect::<Vec<_>>(),
            vec!["!new:example.org", "!mid:example.org", "!old:example.org"]
        );
    }
}
