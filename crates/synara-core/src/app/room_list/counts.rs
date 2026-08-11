//! Aggregate badge / tab counts from a room-list projection (P4.3).
//!
//! Privacy-safe counters only — no room ids, tokens, or display names.

use serde::{Deserialize, Serialize};

use crate::dto::{Membership, RoomSummary};

use super::filters::{room_matches_scope, RoomListScope};

/// Tab / nav badge counters derived from room summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomListBadgeCounts {
    pub joined: u32,
    pub invites: u32,
    pub unread_rooms: u32,
    pub mention_rooms: u32,
    pub direct: u32,
    /// Sum of per-room unread_count across joined rooms (saturates at u32::MAX).
    pub unread_messages: u32,
    /// Sum of per-room highlight_count across joined rooms.
    pub highlight_messages: u32,
    /// Joined rooms with marked_unread set.
    pub marked_unread_rooms: u32,
}

impl RoomListBadgeCounts {
    pub fn from_rooms(rooms: &[RoomSummary]) -> Self {
        let mut out = Self::default();
        for room in rooms {
            match room.membership {
                Membership::Join => {
                    out.joined = out.joined.saturating_add(1);
                    if room.is_direct {
                        out.direct = out.direct.saturating_add(1);
                    }
                    if room_matches_scope(room, RoomListScope::Unread) {
                        out.unread_rooms = out.unread_rooms.saturating_add(1);
                    }
                    if room.highlight_count > 0 {
                        out.mention_rooms = out.mention_rooms.saturating_add(1);
                    }
                    if room.marked_unread {
                        out.marked_unread_rooms = out.marked_unread_rooms.saturating_add(1);
                    }
                    out.unread_messages = out.unread_messages.saturating_add(room.unread_count);
                    out.highlight_messages =
                        out.highlight_messages.saturating_add(room.highlight_count);
                }
                Membership::Invite => {
                    out.invites = out.invites.saturating_add(1);
                }
                Membership::Knock | Membership::Leave | Membership::Ban => {}
            }
        }
        out
    }

    /// Home-tab attention: unread-scope joined rooms + open invites.
    pub fn attention_total(self) -> u32 {
        self.unread_rooms.saturating_add(self.invites)
    }
}
