//! Widget / Element Call session DTO.
//!
//! Prefer host-side URL construction later; wire DTO may omit secrets/query tokens.

use serde::{Deserialize, Serialize};

use super::ids::{RoomId, WidgetId};

/// Widget product kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    ElementCall,
    Custom,
}

impl WidgetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ElementCall => "element_call",
            Self::Custom => "custom",
        }
    }
}

/// Widget / call session lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetSessionState {
    Idle,
    Creating,
    Active,
    Ending,
    Failed,
}

impl WidgetSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Creating => "creating",
            Self::Active => "active",
            Self::Ending => "ending",
            Self::Failed => "failed",
        }
    }
}

/// Element Call / custom widget session projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetSession {
    pub widget_id: WidgetId,
    pub room_id: RoomId,
    pub kind: WidgetKind,
    pub state: WidgetSessionState,
    /// Optional URL — must not embed access tokens or recovery material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub has_active_call: bool,
}
