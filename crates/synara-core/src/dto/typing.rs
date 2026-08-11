//! Typing notification DTO — bounded currently-typing user list.

use serde::{Deserialize, Serialize};

use super::ids::{RoomId, UserId};

/// Currently typing users for a room (product projection).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingSnapshot {
    pub room_id: RoomId,
    /// Bounded list of user ids currently typing.
    pub user_ids: Vec<UserId>,
}
