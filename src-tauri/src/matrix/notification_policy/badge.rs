//! Session-generation-stamped badge counts.

use std::collections::HashMap;

use crate::matrix::dto::RoomId;

/// Saturating total and per-room badge counts.
///
/// Counts are identifiers and `u32` values only. They do not contain event
/// content or notification previews.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BadgeCounter {
    session_generation: u64,
    total: u32,
    by_room: HashMap<RoomId, u32>,
}

impl BadgeCounter {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            total: 0,
            by_room: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn total(&self) -> u32 {
        self.total
    }

    pub fn room_count(&self, room_id: &str) -> u32 {
        self.by_room.get(room_id).copied().unwrap_or(0)
    }

    pub fn tracked_rooms(&self) -> usize {
        self.by_room.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_room.is_empty()
    }

    /// Increment a room by one, saturating both exposed count levels at
    /// [`u32::MAX`].
    pub fn increment(&mut self, room_id: impl Into<RoomId>) -> u32 {
        self.increment_by(room_id, 1)
    }

    /// Increment a room by `amount`, saturating at [`u32::MAX`].
    pub fn increment_by(&mut self, room_id: impl Into<RoomId>, amount: u32) -> u32 {
        let room_id = room_id.into();
        if amount == 0 {
            return self.room_count(&room_id);
        }
        let room_count = self.by_room.entry(room_id).or_default();
        *room_count = room_count.saturating_add(amount);
        let updated = *room_count;
        self.recompute_total();
        updated
    }

    /// Decrement a room by one, flooring at zero and dropping empty entries.
    pub fn decrement(&mut self, room_id: &str) -> u32 {
        self.decrement_by(room_id, 1)
    }

    /// Decrement a room by `amount`, flooring at zero.
    pub fn decrement_by(&mut self, room_id: &str, amount: u32) -> u32 {
        let Some(room_count) = self.by_room.get_mut(room_id) else {
            return 0;
        };
        *room_count = room_count.saturating_sub(amount);
        let updated = *room_count;
        if updated == 0 {
            self.by_room.remove(room_id);
        }
        self.recompute_total();
        updated
    }

    /// Clear one room, returning the count that was removed.
    pub fn clear_room(&mut self, room_id: &str) -> u32 {
        let removed = self.by_room.remove(room_id).unwrap_or(0);
        self.recompute_total();
        removed
    }

    /// Clear all badge counts without changing the session generation.
    pub fn clear(&mut self) {
        self.by_room.clear();
        self.total = 0;
    }

    /// Move to a new session generation and discard all prior-account counts.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }

    fn recompute_total(&mut self) {
        self.total = self
            .by_room
            .values()
            .copied()
            .fold(0_u32, u32::saturating_add);
    }
}
