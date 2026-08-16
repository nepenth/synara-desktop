//! Per-user presence index (P4.7 harness foundation).
//!
//! Pure projection of Matrix presence state for product UI. No SDK presence
//! subscribe/send, no dual-backend, no tokens in errors. Status messages are
//! optional plain text only — never secrets.

use std::collections::HashMap;

use crate::dto::UserId;
use matrix_sdk::ruma::UserId as RumaUserId;

use super::error::PresenceError;

/// Soft cap on tracked users (UI/list safety).
pub const MAX_PRESENCE_USERS: usize = 512;

/// Soft cap on optional status message length (chars).
pub const MAX_STATUS_MSG_CHARS: usize = 256;

/// Largest millisecond timestamp that can cross the IPC boundary without
/// losing integer precision in JavaScript.
pub const MAX_PRESENCE_TIMESTAMP_MS: u64 = 9_007_199_254_740_991;

/// Matrix-aligned presence availability (product projection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresenceState {
    Unknown,
    Offline,
    Online,
    Unavailable,
}

impl PresenceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Offline => "offline",
            Self::Online => "online",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Online | Self::Unavailable)
    }
}

/// Privacy-safe presence snapshot for one user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSnapshot {
    pub user_id: UserId,
    pub state: PresenceState,
    pub currently_active: bool,
    /// Optional last active timestamp (ms). Never a secret.
    pub last_active_ts: Option<u64>,
    /// Optional free-text status; host must not put tokens here.
    pub status_msg: Option<String>,
}

/// Session-generation-stamped presence index.
#[derive(Debug, Default)]
pub struct PresenceIndex {
    session_generation: u64,
    by_user: HashMap<UserId, PresenceSnapshot>,
}

impl PresenceIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_user: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_user.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_user.is_empty()
    }

    fn validate_user(user_id: &str) -> Result<(), PresenceError> {
        Self::validate_user_id(user_id)
    }

    fn validate_status_msg(msg: &Option<String>) -> Result<(), PresenceError> {
        if let Some(s) = msg {
            if s.chars().count() > MAX_STATUS_MSG_CHARS {
                return Err(PresenceError::Invalid {
                    diagnostic_id: "p4.7-status-msg-cap",
                });
            }
        }
        Ok(())
    }

    fn validate_last_active_ts(last_active_ts: Option<u64>) -> Result<(), PresenceError> {
        if last_active_ts.is_some_and(|timestamp| timestamp > MAX_PRESENCE_TIMESTAMP_MS) {
            return Err(PresenceError::Invalid {
                diagnostic_id: "p4.7-last-active-ts-invalid",
            });
        }
        Ok(())
    }

    /// Upsert presence for a user. Returns the stored snapshot.
    pub fn set(
        &mut self,
        user_id: impl Into<String>,
        state: PresenceState,
        currently_active: bool,
        last_active_ts: Option<u64>,
        status_msg: Option<String>,
    ) -> Result<PresenceSnapshot, PresenceError> {
        let user_id = user_id.into().trim().to_owned();
        Self::validate_user(&user_id)?;
        Self::validate_status_msg(&status_msg)?;
        Self::validate_last_active_ts(last_active_ts)?;
        if !self.by_user.contains_key(&user_id) && self.by_user.len() >= MAX_PRESENCE_USERS {
            return Err(PresenceError::Invalid {
                diagnostic_id: "p4.7-presence-user-cap",
            });
        }
        let snap = PresenceSnapshot {
            user_id: user_id.clone(),
            state,
            currently_active,
            last_active_ts,
            status_msg,
        };
        self.by_user.insert(user_id, snap.clone());
        Ok(snap)
    }

    /// Validate a fully-qualified Matrix user ID at the native product boundary.
    fn validate_user_id(user_id: &str) -> Result<(), PresenceError> {
        RumaUserId::parse(user_id)
            .map(|_| ())
            .map_err(|_| PresenceError::Invalid {
                diagnostic_id: "p4.7-invalid-user-id",
            })
    }

    /// Remove presence for one user (idempotent).
    pub fn remove(&mut self, user_id: &str) -> Result<(), PresenceError> {
        Self::validate_user(user_id)?;
        self.by_user.remove(user_id);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.by_user.clear();
    }

    pub fn get(&self, user_id: &str) -> Option<&PresenceSnapshot> {
        self.by_user.get(user_id)
    }

    pub fn state_of(&self, user_id: &str) -> PresenceState {
        self.by_user
            .get(user_id)
            .map(|s| s.state)
            .unwrap_or(PresenceState::Unknown)
    }

    /// Users currently Online or Unavailable, sorted by user_id.
    pub fn active_user_ids(&self) -> Vec<UserId> {
        let mut ids: Vec<UserId> = self
            .by_user
            .values()
            .filter(|s| s.state.is_active())
            .map(|s| s.user_id.clone())
            .collect();
        ids.sort();
        ids
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_user.clear();
    }
}
