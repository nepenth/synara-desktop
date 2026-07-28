//! Ordered desktop notification candidate stream (P9.2 harness foundation).
//!
//! Candidates contain identifiers and routing metadata only. Event plaintext,
//! previews, OS notification delivery, and focus policy belong outside this
//! pure in-memory stream.

use std::collections::VecDeque;

use crate::matrix::dto::{EventId, RoomId};

use super::error::NotificationError;

/// Maximum candidates retained by the stream.
pub const MAX_NOTIFICATION_STREAM_CANDIDATES: usize = 128;

/// Privacy-safe classification used by desktop notification policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateKind {
    Message,
    Mention,
    Invite,
    Other,
}

/// One ordered desktop notification candidate.
///
/// `sequence` is assigned by [`NotificationCandidateStream`] when the
/// candidate is pushed or upserted. No event body or preview is retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub room_id: RoomId,
    pub event_id: EventId,
    pub kind: CandidateKind,
    pub suppressed: bool,
    pub sequence: u64,
}

impl Candidate {
    pub fn new(room_id: RoomId, event_id: EventId, kind: CandidateKind) -> Self {
        Self {
            room_id,
            event_id,
            kind,
            suppressed: false,
            sequence: 0,
        }
    }
}

/// Session-generation-scoped ordered stream of notification candidates.
#[derive(Debug, Default)]
pub struct NotificationCandidateStream {
    session_generation: u64,
    candidates: VecDeque<Candidate>,
    next_sequence: u64,
}

impl NotificationCandidateStream {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            candidates: VecDeque::new(),
            next_sequence: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    fn validate(candidate: &Candidate) -> Result<(), NotificationError> {
        if candidate.room_id.is_empty() || !candidate.room_id.starts_with('!') {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p9.2-invalid-room-id",
            });
        }
        if candidate.event_id.is_empty() || !candidate.event_id.starts_with('$') {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p9.2-invalid-event-id",
            });
        }
        Ok(())
    }

    fn allocate_sequence(&mut self) -> Result<u64, NotificationError> {
        self.next_sequence =
            self.next_sequence
                .checked_add(1)
                .ok_or(NotificationError::Invalid {
                    diagnostic_id: "p9.2-sequence-exhausted",
                })?;
        Ok(self.next_sequence)
    }

    fn position(&self, room_id: &str, event_id: &str) -> Option<usize> {
        self.candidates
            .iter()
            .position(|candidate| candidate.room_id == room_id && candidate.event_id == event_id)
    }

    fn append_with_sequence(&mut self, mut candidate: Candidate, sequence: u64) -> u64 {
        candidate.sequence = sequence;
        self.candidates.push_back(candidate);
        if self.candidates.len() > MAX_NOTIFICATION_STREAM_CANDIDATES {
            self.candidates.pop_front();
        }
        sequence
    }

    /// Append a new candidate. Duplicate `(room_id, event_id)` keys are rejected.
    ///
    /// The incoming `sequence` is ignored; the stream assigns the next sequence.
    pub fn push(&mut self, candidate: Candidate) -> Result<u64, NotificationError> {
        Self::validate(&candidate)?;
        if self
            .position(&candidate.room_id, &candidate.event_id)
            .is_some()
        {
            return Err(NotificationError::Invalid {
                diagnostic_id: "p9.2-duplicate-candidate",
            });
        }
        let sequence = self.allocate_sequence()?;
        Ok(self.append_with_sequence(candidate, sequence))
    }

    /// Insert or replace a candidate and place it at the newest stream position.
    ///
    /// The incoming `sequence` is ignored; every upsert receives a new sequence.
    pub fn upsert(&mut self, candidate: Candidate) -> Result<u64, NotificationError> {
        Self::validate(&candidate)?;
        let sequence = self.allocate_sequence()?;
        if let Some(position) = self.position(&candidate.room_id, &candidate.event_id) {
            self.candidates.remove(position);
        }
        Ok(self.append_with_sequence(candidate, sequence))
    }

    /// Set the suppression hook without changing the candidate's sequence/order.
    pub fn mark_suppressed(&mut self, room_id: &str, event_id: &str, suppressed: bool) -> bool {
        let Some(position) = self.position(room_id, event_id) else {
            return false;
        };
        self.candidates[position].suppressed = suppressed;
        true
    }

    /// Return up to `cap` most recent candidates, oldest-to-newest.
    pub fn list_recent(&self, cap: usize) -> Vec<&Candidate> {
        let skip = self.candidates.len().saturating_sub(cap);
        self.candidates.iter().skip(skip).collect()
    }

    /// Bump generation and wipe all account-scoped stream state.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.candidates.clear();
        self.next_sequence = 0;
    }
}
