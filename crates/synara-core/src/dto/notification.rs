//! OS notification candidate DTO — privacy-filtered title/body only.

use serde::{Deserialize, Serialize};

use super::ids::{EventId, NotificationCandidateId, RoomId};

/// Product notification kind (maps to desktop notification routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    Message,
    Invite,
    AgentApproval,
    LaterReminder,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Invite => "invite",
            Self::AgentApproval => "agent_approval",
            Self::LaterReminder => "later_reminder",
        }
    }
}

/// Candidate for an OS-level desktop notification.
///
/// `title` / `body` are already privacy-filtered product strings — never raw
/// ciphertext or full event dumps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationCandidate {
    pub candidate_id: NotificationCandidateId,
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    /// Product deep-link route (e.g. `/home/room/!id:server`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    pub suppress_if_focused_room: bool,
    pub is_encrypted: bool,
}
