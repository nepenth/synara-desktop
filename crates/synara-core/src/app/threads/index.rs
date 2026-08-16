//! Thread summary index (P5.8 harness foundation).
//!
//! Pure projection of Synara [`ThreadSummary`] DTOs. No SDK thread APIs,
//! no dual-backend, no tokens in errors.

use std::collections::HashMap;

use crate::dto::{EventId, RoomId, ThreadSummary};

use super::error::ThreadError;

/// Soft cap on tracked thread roots per room (UI/list safety).
pub const MAX_THREADS_PER_ROOM: usize = 256;

/// Session-generation-stamped thread summary index.
#[derive(Debug, Default)]
pub struct ThreadIndex {
    session_generation: u64,
    /// room_id → (root_event_id → ThreadSummary)
    by_room: HashMap<RoomId, HashMap<EventId, ThreadSummary>>,
}

impl ThreadIndex {
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

    pub fn thread_count(&self) -> usize {
        self.by_room.values().map(|m| m.len()).sum()
    }

    fn validate_summary(s: &ThreadSummary) -> Result<(), ThreadError> {
        if s.room_id.is_empty() || !s.room_id.starts_with('!') {
            return Err(ThreadError::Invalid {
                diagnostic_id: "p5.8-invalid-room-id",
            });
        }
        if s.root_event_id.is_empty() || !s.root_event_id.starts_with('$') {
            return Err(ThreadError::Invalid {
                diagnostic_id: "p5.8-invalid-root-event-id",
            });
        }
        if let Some(latest) = &s.latest_event_id {
            if latest.is_empty() || !latest.starts_with('$') {
                return Err(ThreadError::Invalid {
                    diagnostic_id: "p5.8-invalid-latest-event-id",
                });
            }
        }
        Ok(())
    }

    /// Upsert one thread summary (host maps SDK → DTO).
    pub fn upsert(&mut self, summary: ThreadSummary) -> Result<(), ThreadError> {
        Self::validate_summary(&summary)?;
        let room = self.by_room.entry(summary.room_id.clone()).or_default();
        if !room.contains_key(&summary.root_event_id) && room.len() >= MAX_THREADS_PER_ROOM {
            return Err(ThreadError::Invalid {
                diagnostic_id: "p5.8-thread-cap",
            });
        }
        room.insert(summary.root_event_id.clone(), summary);
        Ok(())
    }

    /// Upsert many summaries; first error aborts after applying none of the remaining.
    pub fn upsert_batch(&mut self, summaries: Vec<ThreadSummary>) -> Result<usize, ThreadError> {
        let mut n = 0;
        for s in summaries {
            self.upsert(s)?;
            n += 1;
        }
        Ok(n)
    }

    pub fn get(&self, room_id: &str, root_event_id: &str) -> Option<&ThreadSummary> {
        self.by_room.get(room_id)?.get(root_event_id)
    }

    /// List thread summaries for a room, newest activity first.
    pub fn list_room(&self, room_id: &str) -> Vec<&ThreadSummary> {
        match self.by_room.get(room_id) {
            Some(map) => {
                let mut v: Vec<_> = map.values().collect();
                v.sort_by(|a, b| {
                    b.latest_origin_server_ts
                        .cmp(&a.latest_origin_server_ts)
                        .then_with(|| a.root_event_id.cmp(&b.root_event_id))
                });
                v
            }
            None => Vec::new(),
        }
    }

    /// Roots the local user participated in (participated == true).
    pub fn list_participated(&self, room_id: &str) -> Vec<&ThreadSummary> {
        self.list_room(room_id)
            .into_iter()
            .filter(|s| s.participated)
            .collect()
    }

    pub fn remove(&mut self, room_id: &str, root_event_id: &str) -> bool {
        let Some(map) = self.by_room.get_mut(room_id) else {
            return false;
        };
        let removed = map.remove(root_event_id).is_some();
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
