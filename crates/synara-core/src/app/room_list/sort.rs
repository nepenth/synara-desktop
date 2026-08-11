//! Room-list ordering helpers (P4.4 recent + favorite priority).
//!
//! Pure sorts over [`RoomSummary`] — no SDK types.

use crate::dto::RoomSummary;

/// Sort keys for product room lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomListSort {
    /// Newest `last_activity_ts` first; missing ts sorts last; stable by room_id.
    RecentActivity,
    /// Favorites first, then recent activity among the rest.
    FavoritesThenRecent,
    /// Low-priority rooms last, then recent activity.
    LowPriorityLast,
}

impl RoomListSort {
    pub fn as_str(self) -> &'static str {
        match self {
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
        let by_recent =
            |x: &RoomSummary, y: &RoomSummary| match (x.last_activity_ts, y.last_activity_ts) {
                (Some(tx), Some(ty)) => ty.cmp(&tx), // newer first
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => x.room_id.cmp(&y.room_id),
            };
        match sort {
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
