//! Per-room typing snapshot index (P6.3 harness foundation).
//!
//! Pure projection of Synara [`TypingSnapshot`] DTOs. No SDK typing send,
//! no dual-backend, no tokens in errors.

use std::collections::{BTreeSet, HashMap};

use crate::dto::{RoomId, TypingSnapshot, UserId};

use super::error::TypingError;

/// Soft cap on users reported as typing in one room (UI safety).
pub const MAX_TYPING_USERS_PER_ROOM: usize = 32;

/// Session-generation-stamped typing index.
#[derive(Debug, Default)]
pub struct TypingIndex {
    session_generation: u64,
    by_room: HashMap<RoomId, BTreeSet<UserId>>,
}

impl TypingIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_room: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn room_count(&self) -> usize {
        self.by_room.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_room.is_empty()
    }

    fn validate_room(room_id: &str) -> Result<(), TypingError> {
        if room_id.is_empty() || !room_id.starts_with('!') {
            return Err(TypingError::Invalid {
                diagnostic_id: "p6.3-invalid-room-id",
            });
        }
        Ok(())
    }

    fn validate_user(user_id: &str) -> Result<(), TypingError> {
        if user_id.is_empty() || !user_id.starts_with('@') {
            return Err(TypingError::Invalid {
                diagnostic_id: "p6.3-invalid-user-id",
            });
        }
        Ok(())
    }

    /// Replace the typing set for a room (host applies ephemeral events).
    pub fn set_users(
        &mut self,
        room_id: impl Into<String>,
        user_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<TypingSnapshot, TypingError> {
        let room_id = room_id.into().trim().to_owned();
        Self::validate_room(&room_id)?;
        let mut set = BTreeSet::new();
        for u in user_ids {
            let uid = u.into().trim().to_owned();
            Self::validate_user(&uid)?;
            set.insert(uid);
            if set.len() > MAX_TYPING_USERS_PER_ROOM {
                return Err(TypingError::Invalid {
                    diagnostic_id: "p6.3-typing-user-cap",
                });
            }
        }
        if set.is_empty() {
            self.by_room.remove(&room_id);
        } else {
            self.by_room.insert(room_id.clone(), set);
        }
        Ok(self.snapshot(&room_id))
    }

    /// Add one user as typing (idempotent).
    pub fn add_user(
        &mut self,
        room_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<TypingSnapshot, TypingError> {
        let room_id = room_id.into().trim().to_owned();
        let user_id = user_id.into().trim().to_owned();
        Self::validate_room(&room_id)?;
        Self::validate_user(&user_id)?;
        let set = self.by_room.entry(room_id.clone()).or_default();
        if !set.contains(&user_id) && set.len() >= MAX_TYPING_USERS_PER_ROOM {
            return Err(TypingError::Invalid {
                diagnostic_id: "p6.3-typing-user-cap",
            });
        }
        set.insert(user_id);
        Ok(self.snapshot(&room_id))
    }

    /// Remove one user from typing (idempotent if missing).
    pub fn remove_user(
        &mut self,
        room_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<TypingSnapshot, TypingError> {
        let room_id = room_id.into().trim().to_owned();
        let user_id = user_id.into().trim().to_owned();
        Self::validate_room(&room_id)?;
        Self::validate_user(&user_id)?;
        if let Some(set) = self.by_room.get_mut(&room_id) {
            set.remove(&user_id);
            if set.is_empty() {
                self.by_room.remove(&room_id);
            }
        }
        Ok(self.snapshot(&room_id))
    }

    /// Clear typing for one room.
    pub fn clear_room(&mut self, room_id: &str) -> Result<(), TypingError> {
        Self::validate_room(room_id)?;
        self.by_room.remove(room_id);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.by_room.clear();
    }

    /// Privacy-safe snapshot for IPC/UI.
    pub fn snapshot(&self, room_id: &str) -> TypingSnapshot {
        let user_ids = self
            .by_room
            .get(room_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        TypingSnapshot {
            room_id: room_id.to_owned(),
            user_ids,
        }
    }

    pub fn is_typing(&self, room_id: &str, user_id: &str) -> bool {
        self.by_room
            .get(room_id)
            .map(|s| s.contains(user_id))
            .unwrap_or(false)
    }

    /// All rooms currently reporting at least one typer.
    pub fn nonempty_snapshots(&self) -> Vec<TypingSnapshot> {
        let mut rooms: Vec<TypingSnapshot> = self
            .by_room
            .keys()
            .map(|room_id| self.snapshot(room_id))
            .collect();
        rooms.sort_by(|a, b| a.room_id.cmp(&b.room_id));
        rooms
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_room.clear();
    }
}
