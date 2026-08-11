//! Session snapshot DTO — product projection of the live Matrix session.
//!
//! **Never** includes `access_token`, `refresh_token`, recovery keys, or
//! passwords. Tokens remain host-side only.

use serde::{Deserialize, Serialize};

use super::ids::{DeviceId, UserId};

/// High-level session supervisor lifecycle (product meaning; not an SDK enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLifecycle {
    Empty,
    Opening,
    Authenticating,
    Restoring,
    Syncing,
    Ready,
    Stopping,
    LoggedOut,
    Failed,
    Wiping,
}

impl SessionLifecycle {
    pub const ALL: &'static [SessionLifecycle] = &[
        Self::Empty,
        Self::Opening,
        Self::Authenticating,
        Self::Restoring,
        Self::Syncing,
        Self::Ready,
        Self::Stopping,
        Self::LoggedOut,
        Self::Failed,
        Self::Wiping,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Opening => "opening",
            Self::Authenticating => "authenticating",
            Self::Restoring => "restoring",
            Self::Syncing => "syncing",
            Self::Ready => "ready",
            Self::Stopping => "stopping",
            Self::LoggedOut => "logged_out",
            Self::Failed => "failed",
            Self::Wiping => "wiping",
        }
    }
}

/// Product session projection for IPC snapshots / status streams.
///
/// Wire JSON: camelCase fields. No session tokens on this DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session_generation: u64,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub homeserver_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// mxc URI or product media-handle URI — string only, never bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub lifecycle: SessionLifecycle,
    pub crypto_ready: bool,
}
