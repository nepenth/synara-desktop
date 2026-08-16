//! Read-receipt DTO.

use serde::{Deserialize, Serialize};

use super::ids::{EventId, RoomId, UserId};

/// Receipt kind (product enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptType {
    Read,
    ReadPrivate,
    FullyRead,
}

impl ReceiptType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::ReadPrivate => "read_private",
            Self::FullyRead => "fully_read",
        }
    }
}

/// Single receipt projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub room_id: RoomId,
    pub event_id: EventId,
    pub user_id: UserId,
    pub receipt_type: ReceiptType,
    /// Receipt timestamp in milliseconds since Unix epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    /// Thread root when the receipt is thread-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<EventId>,
}
