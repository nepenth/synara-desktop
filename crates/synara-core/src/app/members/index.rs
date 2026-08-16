//! Room member list index (P4.6 harness foundation).
//!
//! Pure projection of Synara [`RoomMember`] DTOs. No SDK member APIs,
//! no dual-backend, no tokens in errors.

use std::collections::HashMap;

use crate::dto::{Membership, RoomId, RoomMember, UserId};

use super::error::MemberError;

/// Soft cap on members tracked per room (list safety / memory bound).
pub const MAX_MEMBERS_PER_ROOM: usize = 4096;

/// Session-generation-stamped room member index.
#[derive(Debug, Default)]
pub struct MemberIndex {
    session_generation: u64,
    /// room_id → (user_id → RoomMember)
    by_room: HashMap<RoomId, HashMap<UserId, RoomMember>>,
}

impl MemberIndex {
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

    pub fn member_count(&self, room_id: &str) -> usize {
        self.by_room.get(room_id).map(|m| m.len()).unwrap_or(0)
    }

    fn validate_member(m: &RoomMember) -> Result<(), MemberError> {
        if m.room_id.is_empty() || !m.room_id.starts_with('!') {
            return Err(MemberError::Invalid {
                diagnostic_id: "p4.6-invalid-room-id",
            });
        }
        if m.user_id.is_empty() || !m.user_id.starts_with('@') {
            return Err(MemberError::Invalid {
                diagnostic_id: "p4.6-invalid-user-id",
            });
        }
        Ok(())
    }

    /// Upsert one member (host maps SDK → DTO).
    pub fn upsert(&mut self, member: RoomMember) -> Result<(), MemberError> {
        Self::validate_member(&member)?;
        let room = self.by_room.entry(member.room_id.clone()).or_default();
        if !room.contains_key(&member.user_id) && room.len() >= MAX_MEMBERS_PER_ROOM {
            return Err(MemberError::Invalid {
                diagnostic_id: "p4.6-member-cap",
            });
        }
        room.insert(member.user_id.clone(), member);
        Ok(())
    }

    pub fn upsert_batch(&mut self, members: Vec<RoomMember>) -> Result<usize, MemberError> {
        let mut n = 0;
        for m in members {
            self.upsert(m)?;
            n += 1;
        }
        Ok(n)
    }

    pub fn get(&self, room_id: &str, user_id: &str) -> Option<&RoomMember> {
        self.by_room.get(room_id)?.get(user_id)
    }

    /// Members in `room_id` filtered by optional membership, sorted by user_id.
    pub fn list_room(&self, room_id: &str, membership: Option<Membership>) -> Vec<&RoomMember> {
        match self.by_room.get(room_id) {
            Some(map) => {
                let mut v: Vec<_> = map
                    .values()
                    .filter(|m| membership.is_none_or(|want| m.membership == want))
                    .collect();
                v.sort_by(|a, b| {
                    // Higher power first, then user_id for stability.
                    b.power_level
                        .cmp(&a.power_level)
                        .then_with(|| a.user_id.cmp(&b.user_id))
                });
                v
            }
            None => Vec::new(),
        }
    }

    /// Joined members only (common list default).
    pub fn list_joined(&self, room_id: &str) -> Vec<&RoomMember> {
        self.list_room(room_id, Some(Membership::Join))
    }

    /// Highest power_level member in room (ties: lexicographically first user_id).
    pub fn highest_power(&self, room_id: &str) -> Option<&RoomMember> {
        self.list_room(room_id, None).into_iter().next()
    }

    pub fn remove(&mut self, room_id: &str, user_id: &str) -> bool {
        let Some(map) = self.by_room.get_mut(room_id) else {
            return false;
        };
        let removed = map.remove(user_id).is_some();
        if map.is_empty() {
            self.by_room.remove(room_id);
        }
        removed
    }

    pub fn clear_room(&mut self, room_id: &str) {
        self.by_room.remove(room_id);
    }

    pub fn clear(&mut self) {
        self.by_room.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_room.clear();
    }
}
