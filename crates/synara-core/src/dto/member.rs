//! Room member DTO — list/drawer projection.

use serde::{Deserialize, Serialize};

use super::ids::{RoomId, UserId};
use super::room::Membership;

/// Single room member projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomMember {
    pub room_id: RoomId,
    pub user_id: UserId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// mxc or product media-handle URI — string only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub membership: Membership,
    pub power_level: i32,
    /// True when this member is the DM peer for a direct room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_direct_target: Option<bool>,
}
