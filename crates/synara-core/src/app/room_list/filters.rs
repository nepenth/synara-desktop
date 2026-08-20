//! Room-list filters for membership / unread / invite / tag views (P4.3–P4.4).
//!
//! Pure predicates over [`RoomSummary`] — no SDK Room objects, no network.

use crate::dto::{Membership, RoomSummary};

/// Product room-list scope filters used by nav tabs (subset of iOS/desktop scopes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomListScope {
    /// Joined rooms only (default home list).
    Joined,
    /// Invite membership only.
    Invites,
    /// Rooms with unread_count > 0, highlight_count > 0, or marked_unread.
    Unread,
    /// Highlight/mention count > 0.
    Mentions,
    /// Direct-message joined rooms.
    Direct,
    /// Favorite-tagged joined rooms (`m.favourite`).
    Favorites,
    /// Low-priority-tagged joined rooms (`m.lowpriority`).
    LowPriority,
    /// All memberships except ban (includes leave/knock for recovery UIs).
    AllActive,
}

impl RoomListScope {
    pub const ALL: &'static [RoomListScope] = &[
        Self::Joined,
        Self::Invites,
        Self::Unread,
        Self::Mentions,
        Self::Direct,
        Self::Favorites,
        Self::LowPriority,
        Self::AllActive,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Joined => "joined",
            Self::Invites => "invites",
            Self::Unread => "unread",
            Self::Mentions => "mentions",
            Self::Direct => "direct",
            Self::Favorites => "favorites",
            Self::LowPriority => "low_priority",
            Self::AllActive => "all_active",
        }
    }
}

/// Whether a room row is visible under `scope`.
pub fn room_matches_scope(room: &RoomSummary, scope: RoomListScope) -> bool {
    match scope {
        RoomListScope::Joined => room.membership == Membership::Join,
        RoomListScope::Invites => room.membership == Membership::Invite,
        RoomListScope::Unread => {
            room.membership == Membership::Join
                && (room.unread_count > 0 || room.highlight_count > 0 || room.marked_unread)
        }
        RoomListScope::Mentions => room.membership == Membership::Join && room.highlight_count > 0,
        RoomListScope::Direct => room.membership == Membership::Join && room.is_direct,
        RoomListScope::Favorites => room.membership == Membership::Join && room.is_favorite,
        RoomListScope::LowPriority => room.membership == Membership::Join && room.is_low_priority,
        RoomListScope::AllActive => !matches!(room.membership, Membership::Ban),
    }
}

/// Rooms in an explicit product folder (exact folder_id match).
pub fn select_rooms_in_folder(rooms: &[RoomSummary], folder_id: &str) -> Vec<RoomSummary> {
    rooms
        .iter()
        .filter(|r| r.folder_id.as_deref() == Some(folder_id))
        .cloned()
        .collect()
}

/// Filter an ordered projection slice by scope (preserves order).
pub fn filter_rooms_by_scope<'a>(
    rooms: impl IntoIterator<Item = &'a RoomSummary>,
    scope: RoomListScope,
) -> Vec<&'a RoomSummary> {
    rooms
        .into_iter()
        .filter(|r| room_matches_scope(r, scope))
        .collect()
}

/// Owned clone of rooms matching scope (for snapshot bodies).
pub fn select_rooms_by_scope(rooms: &[RoomSummary], scope: RoomListScope) -> Vec<RoomSummary> {
    rooms
        .iter()
        .filter(|r| room_matches_scope(r, scope))
        .cloned()
        .collect()
}

/// Split rooms into favorite-tagged joined rooms and the remaining list.
///
/// Favorites are joined rooms with `m.favourite`. Remaining rooms keep their
/// original relative order (including invites).
pub fn partition_favorite_rooms(rooms: &[RoomSummary]) -> (Vec<RoomSummary>, Vec<RoomSummary>) {
    let mut favorites = Vec::new();
    let mut remaining = Vec::new();
    for room in rooms {
        if room_matches_scope(room, RoomListScope::Favorites) {
            favorites.push(room.clone());
        } else {
            remaining.push(room.clone());
        }
    }
    (favorites, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::room_list::summary::RoomSummaryBuilder;

    #[test]
    fn favorites_partition_excludes_tagged_rooms_from_remaining() {
        let rooms = vec![
            RoomSummaryBuilder::new("!fav:example.org")
                .name("Fav")
                .favorite(true)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!plain:example.org")
                .name("Plain")
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!invite:example.org")
                .name("Invite")
                .membership(Membership::Invite)
                .favorite(true)
                .build()
                .unwrap(),
        ];
        let (favorites, remaining) = partition_favorite_rooms(&rooms);
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].room_id.as_str(), "!fav:example.org");
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].room_id.as_str(), "!plain:example.org");
        assert_eq!(remaining[1].room_id.as_str(), "!invite:example.org");
    }
}
