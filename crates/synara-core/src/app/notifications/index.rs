//! Notification candidate index (P7.1 foundation).
//!
//! Pure queue/index of privacy-filtered [`NotificationCandidate`] DTOs owned
//! in production by the notification decision owner. No OS notification
//! posting, no dual-backend, no tokens in errors.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::dto::{NotificationCandidate, NotificationCandidateId, RoomId};

use super::error::NotificationError;

/// Soft cap on pending candidates (memory / UI backlog bound).
pub const MAX_PENDING_CANDIDATES: usize = 128;

/// Session-generation-stamped notification candidate index.
#[derive(Debug, Default)]
pub struct NotificationIndex {
    session_generation: u64,
    /// Insertion order of candidate ids still pending.
    order: VecDeque<NotificationCandidateId>,
    by_id: HashMap<NotificationCandidateId, NotificationCandidate>,
    /// Dedup keys: (room_id, event_id) when event present.
    seen_events: HashSet<(RoomId, String)>,
    next_seq: u64,
    /// Currently focused room (suppress_if_focused_room honor).
    focused_room_id: Option<RoomId>,
}

impl NotificationIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            order: VecDeque::new(),
            by_id: HashMap::new(),
            seen_events: HashSet::new(),
            next_seq: 0,
            focused_room_id: None,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn set_focused_room(&mut self, room_id: Option<RoomId>) {
        self.focused_room_id = room_id;
    }

    pub fn focused_room(&self) -> Option<&str> {
        self.focused_room_id.as_deref()
    }

    /// Whether this `(room_id, event_id)` pair already notified. Used by the
    /// decision owner to report the exact suppress reason without leaking
    /// identifiers. Events without an id are never duplicates.
    pub fn is_duplicate(&self, room_id: &str, event_id: &str) -> bool {
        self.seen_events
            .contains(&(room_id.to_owned(), event_id.to_owned()))
    }

    fn validate(c: &NotificationCandidate) -> Result<(), NotificationError> {
        if c.candidate_id.is_empty() {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p7.1-empty-candidate-id",
            });
        }
        if c.room_id.is_empty() || !c.room_id.starts_with('!') {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p7.1-invalid-room-id",
            });
        }
        if c.title.is_empty() && c.body.is_empty() {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p7.1-empty-title-and-body",
            });
        }
        if let Some(ev) = &c.event_id {
            if ev.is_empty() || !ev.starts_with('$') {
                return Err(NotificationError::Invalid {
                    diagnostic_id: "p7.1-invalid-event-id",
                });
            }
        }
        Ok(())
    }

    fn alloc_id(&mut self) -> NotificationCandidateId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("notif-{}", self.next_seq)
    }

    /// Enqueue a candidate. Host must pass privacy-filtered title/body only.
    /// Returns `Ok(None)` when suppressed (focused room) or duplicate event.
    pub fn enqueue(
        &mut self,
        mut candidate: NotificationCandidate,
    ) -> Result<Option<NotificationCandidateId>, NotificationError> {
        if candidate.candidate_id.is_empty() {
            candidate.candidate_id = self.alloc_id();
        }
        Self::validate(&candidate)?;

        if candidate.suppress_if_focused_room {
            if let Some(focused) = &self.focused_room_id {
                if focused == &candidate.room_id {
                    return Ok(None);
                }
            }
        }

        if let Some(ev) = &candidate.event_id {
            let key = (candidate.room_id.clone(), ev.clone());
            if self.seen_events.contains(&key) {
                return Ok(None);
            }
            self.seen_events.insert(key);
        }

        if self.by_id.len() >= MAX_PENDING_CANDIDATES {
            // Drop oldest pending to make room.
            if let Some(old) = self.order.pop_front() {
                if let Some(removed) = self.by_id.remove(&old) {
                    if let Some(ev) = removed.event_id {
                        self.seen_events.remove(&(removed.room_id, ev));
                    }
                }
            }
        }

        let id = candidate.candidate_id.clone();
        if self.by_id.contains_key(&id) {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p7.1-duplicate-candidate-id",
            });
        }
        self.order.push_back(id.clone());
        self.by_id.insert(id.clone(), candidate);
        Ok(Some(id))
    }

    pub fn get(&self, candidate_id: &str) -> Option<&NotificationCandidate> {
        self.by_id.get(candidate_id)
    }

    /// Pending candidates in insertion order.
    pub fn list_pending(&self) -> Vec<&NotificationCandidate> {
        self.order
            .iter()
            .filter_map(|id| self.by_id.get(id))
            .collect()
    }

    /// Acknowledge / dismiss a candidate (posted or user dismissed).
    pub fn dismiss(&mut self, candidate_id: &str) -> bool {
        let Some(removed) = self.by_id.remove(candidate_id) else {
            return false;
        };
        self.order.retain(|id| id != candidate_id);
        if let Some(ev) = removed.event_id {
            // Keep seen_events so we do not re-notify the same event.
            let _ = ev;
        }
        true
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.by_id.clear();
        self.seen_events.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
        self.next_seq = 0;
        self.focused_room_id = None;
    }
}
