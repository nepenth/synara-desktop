//! Room summary DTO — list/nav projection (not a full SDK Room object graph).

use serde::{Deserialize, Serialize};

use super::ids::{RoomId, UserId};

/// Membership of the local user in a room (product enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Membership {
    Invite,
    Join,
    Knock,
    Leave,
    Ban,
}

impl Membership {
    pub const ALL: &'static [Membership] = &[
        Self::Invite,
        Self::Join,
        Self::Knock,
        Self::Leave,
        Self::Ban,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invite => "invite",
            Self::Join => "join",
            Self::Knock => "knock",
            Self::Leave => "leave",
            Self::Ban => "ban",
        }
    }
}

/// Per-room notification preference projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    All,
    Mentions,
    Mute,
    Default,
}

impl NotificationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Mentions => "mentions",
            Self::Mute => "mute",
            Self::Default => "default",
        }
    }
}

/// Bounded hero entry for DM / small-room name fallbacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomHero {
    pub user_id: UserId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Room list / nav summary (product DTO).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSummary {
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_alias: Option<String>,
    /// mxc or product media-handle URI — string only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub membership: Membership,
    pub is_direct: bool,
    /// True when the room is a Matrix space (`m.space`).
    #[serde(default)]
    pub is_space: bool,
    /// True when the room is a Matrix voice room (`m.room.create` type `m.call`).
    #[serde(default)]
    pub is_call: bool,
    /// Account-data favorite (m.tag `m.favourite`) projection.
    #[serde(default)]
    pub is_favorite: bool,
    /// Account-data low-priority (m.tag `m.lowpriority`) projection.
    #[serde(default)]
    pub is_low_priority: bool,
    /// Optional product folder / section label (not a Matrix space id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub is_encrypted: bool,
    /// Stable join-rule string (e.g. `public`, `invite`); not an SDK enum object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_rule: Option<String>,
    pub unread_count: u32,
    pub highlight_count: u32,
    pub marked_unread: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notification_mode: Option<NotificationMode>,
    /// Last activity timestamp in milliseconds since Unix epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_ts: Option<u64>,
    /// Bounded hero list for name/avatar fallbacks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heroes: Option<Vec<RoomHero>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tombstone_successor_room_id: Option<RoomId>,
}
