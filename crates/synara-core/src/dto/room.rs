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

/// Authoritative room-encryption knowledge carried across product boundaries.
///
/// `Unknown` is deliberately distinct from `NotEncrypted`: an incomplete or
/// failed SDK state read must never authorize a cleartext-sensitive action.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomEncryptionStatus {
    Encrypted,
    NotEncrypted,
    #[default]
    Unknown,
}

impl RoomEncryptionStatus {
    pub const fn is_encrypted(self) -> bool {
        matches!(self, Self::Encrypted)
    }
}

/// Room list / nav summary (product DTO).
#[derive(Debug, Clone, PartialEq)]
pub struct RoomSummary {
    pub room_id: RoomId,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    /// mxc or product media-handle URI — string only.
    pub avatar_url: Option<String>,
    pub membership: Membership,
    pub is_direct: bool,
    /// True when the room is a Matrix space (`m.space`).
    pub is_space: bool,
    /// True when the room is a Matrix voice room (`m.room.create` type `m.call`).
    pub is_call: bool,
    /// Account-data favorite (m.tag `m.favourite`) projection.
    pub is_favorite: bool,
    /// Account-data low-priority (m.tag `m.lowpriority`) projection.
    pub is_low_priority: bool,
    /// Optional product folder / section label (not a Matrix space id).
    pub folder_id: Option<String>,
    pub encryption_status: RoomEncryptionStatus,
    /// Stable join-rule string (e.g. `public`, `invite`); not an SDK enum object.
    pub join_rule: Option<String>,
    pub unread_count: u32,
    pub highlight_count: u32,
    pub marked_unread: bool,
    pub notification_mode: Option<NotificationMode>,
    /// Last activity timestamp in milliseconds since Unix epoch.
    pub last_activity_ts: Option<u64>,
    /// Privacy-safe last-message preview. Never tokens or `mxc://`.
    pub last_message_preview: Option<String>,
    /// Bounded hero list for name/avatar fallbacks.
    pub heroes: Option<Vec<RoomHero>>,
    pub tombstone_successor_room_id: Option<RoomId>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RoomSummarySerialize<'a> {
    room_id: &'a RoomId,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonical_alias: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: &'a Option<String>,
    membership: Membership,
    is_direct: bool,
    is_space: bool,
    is_call: bool,
    is_favorite: bool,
    is_low_priority: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder_id: &'a Option<String>,
    /// Backwards-compatible chrome hint, structurally derived from the
    /// authoritative tri-state so Core can never emit contradictory fields.
    is_encrypted: bool,
    encryption_status: RoomEncryptionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    join_rule: &'a Option<String>,
    unread_count: u32,
    highlight_count: u32,
    marked_unread: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification_mode: Option<NotificationMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_activity_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message_preview: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heroes: &'a Option<Vec<RoomHero>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tombstone_successor_room_id: &'a Option<RoomId>,
}

impl Serialize for RoomSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        RoomSummarySerialize {
            room_id: &self.room_id,
            name: &self.name,
            canonical_alias: &self.canonical_alias,
            avatar_url: &self.avatar_url,
            membership: self.membership,
            is_direct: self.is_direct,
            is_space: self.is_space,
            is_call: self.is_call,
            is_favorite: self.is_favorite,
            is_low_priority: self.is_low_priority,
            folder_id: &self.folder_id,
            is_encrypted: self.encryption_status.is_encrypted(),
            encryption_status: self.encryption_status,
            join_rule: &self.join_rule,
            unread_count: self.unread_count,
            highlight_count: self.highlight_count,
            marked_unread: self.marked_unread,
            notification_mode: self.notification_mode,
            last_activity_ts: self.last_activity_ts,
            last_message_preview: &self.last_message_preview,
            heroes: &self.heroes,
            tombstone_successor_room_id: &self.tombstone_successor_room_id,
        }
        .serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoomSummaryWire {
    room_id: RoomId,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    canonical_alias: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    membership: Membership,
    is_direct: bool,
    #[serde(default)]
    is_space: bool,
    #[serde(default)]
    is_call: bool,
    #[serde(default)]
    is_favorite: bool,
    #[serde(default)]
    is_low_priority: bool,
    #[serde(default)]
    folder_id: Option<String>,
    is_encrypted: bool,
    encryption_status: RoomEncryptionStatus,
    #[serde(default)]
    join_rule: Option<String>,
    unread_count: u32,
    highlight_count: u32,
    marked_unread: bool,
    #[serde(default)]
    notification_mode: Option<NotificationMode>,
    #[serde(default)]
    last_activity_ts: Option<u64>,
    #[serde(default)]
    last_message_preview: Option<String>,
    #[serde(default)]
    heroes: Option<Vec<RoomHero>>,
    #[serde(default)]
    tombstone_successor_room_id: Option<RoomId>,
}

impl<'de> Deserialize<'de> for RoomSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = RoomSummaryWire::deserialize(deserializer)?;
        if wire.is_encrypted != wire.encryption_status.is_encrypted() {
            return Err(serde::de::Error::custom(
                "room encryption fields are inconsistent",
            ));
        }
        Ok(Self {
            room_id: wire.room_id,
            name: wire.name,
            canonical_alias: wire.canonical_alias,
            avatar_url: wire.avatar_url,
            membership: wire.membership,
            is_direct: wire.is_direct,
            is_space: wire.is_space,
            is_call: wire.is_call,
            is_favorite: wire.is_favorite,
            is_low_priority: wire.is_low_priority,
            folder_id: wire.folder_id,
            encryption_status: wire.encryption_status,
            join_rule: wire.join_rule,
            unread_count: wire.unread_count,
            highlight_count: wire.highlight_count,
            marked_unread: wire.marked_unread,
            notification_mode: wire.notification_mode,
            last_activity_ts: wire.last_activity_ts,
            last_message_preview: wire.last_message_preview,
            heroes: wire.heroes,
            tombstone_successor_room_id: wire.tombstone_successor_room_id,
        })
    }
}
