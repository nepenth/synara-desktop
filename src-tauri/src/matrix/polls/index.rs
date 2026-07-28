//! Poll + room state/membership projection (P5.7 harness foundation).
//!
//! Pure product indexes. **No poll answer plaintext dumps beyond short labels,
//! no tokens, no dual-backend.** Host maps MSC3381 / state events → these shapes.

use std::collections::{BTreeMap, HashMap};

use crate::matrix::dto::{EventId, RoomId, TimelineMembershipItem, TimelineStateItem, UserId};

use super::error::PollError;

/// Soft caps (UI / memory safety).
pub const MAX_POLLS_PER_ROOM: usize = 256;
pub const MAX_ANSWERS_PER_POLL: usize = 32;
pub const MAX_STATE_KEYS_PER_ROOM: usize = 1_024;
pub const MAX_MEMBERSHIP_EVENTS: usize = 2_048;

/// Lifecycle of a poll start event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PollPhase {
    Open,
    Closed,
}

impl PollPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

/// One poll answer option (short label only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollAnswer {
    pub id: String,
    pub label: String,
}

/// Aggregated poll projection for UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollSummary {
    pub room_id: RoomId,
    pub start_event_id: EventId,
    pub sender: UserId,
    pub origin_server_ts: u64,
    /// Short question text (already privacy-filtered by host).
    pub question: String,
    pub answers: Vec<PollAnswer>,
    pub phase: PollPhase,
    /// answer_id → vote count (no voter user lists by default).
    pub vote_counts: BTreeMap<String, u32>,
    pub total_responses: u32,
    pub end_event_id: Option<EventId>,
}

/// Session-generation-stamped poll index.
#[derive(Debug, Default)]
pub struct PollIndex {
    session_generation: u64,
    /// (room_id, start_event_id) → summary
    polls: HashMap<(RoomId, EventId), PollSummary>,
    /// response event → (room, start) for idempotent replace
    responses: HashMap<(RoomId, EventId), (EventId, UserId, Vec<String>)>,
}

impl PollIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            polls: HashMap::new(),
            responses: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.polls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.polls.is_empty()
    }

    pub fn open_count(&self) -> usize {
        self.polls
            .values()
            .filter(|p| p.phase == PollPhase::Open)
            .count()
    }

    fn validate_event_id(id: &str) -> Result<(), PollError> {
        if id.is_empty() || !id.starts_with('$') {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-invalid-event-id",
            });
        }
        Ok(())
    }

    fn validate_room(id: &str) -> Result<(), PollError> {
        if id.is_empty() || !id.starts_with('!') {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-invalid-room-id",
            });
        }
        Ok(())
    }

    fn validate_user(id: &str) -> Result<(), PollError> {
        if id.is_empty() || !id.starts_with('@') {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-invalid-user-id",
            });
        }
        Ok(())
    }

    fn forbid_secret_text(s: &str) -> Result<(), PollError> {
        let lower = s.to_ascii_lowercase();
        if lower.contains("access_token")
            || lower.contains("refresh_token")
            || lower.contains("private_key")
            || lower.contains("password=")
        {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-forbidden-text",
            });
        }
        Ok(())
    }

    /// Upsert a poll start (host-mapped MSC3381 start).
    pub fn upsert_start(&mut self, mut summary: PollSummary) -> Result<(), PollError> {
        Self::validate_room(&summary.room_id)?;
        Self::validate_event_id(&summary.start_event_id)?;
        Self::validate_user(&summary.sender)?;
        Self::forbid_secret_text(&summary.question)?;
        if summary.answers.is_empty() || summary.answers.len() > MAX_ANSWERS_PER_POLL {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-invalid-answer-count",
            });
        }
        for a in &summary.answers {
            if a.id.is_empty() || a.label.is_empty() {
                return Err(PollError::Invalid {
                    diagnostic_id: "p5.7-invalid-answer",
                });
            }
            Self::forbid_secret_text(&a.label)?;
        }
        let room_polls = self
            .polls
            .keys()
            .filter(|(r, _)| r == &summary.room_id)
            .count();
        let key = (summary.room_id.clone(), summary.start_event_id.clone());
        if !self.polls.contains_key(&key) && room_polls >= MAX_POLLS_PER_ROOM {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-poll-cap",
            });
        }
        // Preserve accumulated votes if re-upserting metadata.
        if let Some(prev) = self.polls.get(&key) {
            summary.vote_counts = prev.vote_counts.clone();
            summary.total_responses = prev.total_responses;
            summary.phase = prev.phase;
            summary.end_event_id = prev.end_event_id.clone();
        } else {
            summary.vote_counts = summary.answers.iter().map(|a| (a.id.clone(), 0)).collect();
            summary.total_responses = 0;
            summary.phase = PollPhase::Open;
            summary.end_event_id = None;
        }
        self.polls.insert(key, summary);
        Ok(())
    }

    /// Record a response (replaces prior response from same user on same poll).
    pub fn apply_response(
        &mut self,
        room_id: impl Into<String>,
        response_event_id: impl Into<String>,
        start_event_id: impl Into<String>,
        sender: impl Into<String>,
        answer_ids: Vec<String>,
    ) -> Result<(), PollError> {
        let room_id = room_id.into();
        let response_event_id = response_event_id.into();
        let start_event_id = start_event_id.into();
        let sender = sender.into();
        Self::validate_room(&room_id)?;
        Self::validate_event_id(&response_event_id)?;
        Self::validate_event_id(&start_event_id)?;
        Self::validate_user(&sender)?;
        if answer_ids.is_empty() {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-empty-response",
            });
        }
        let poll_key = (room_id.clone(), start_event_id.clone());
        let poll = self.polls.get_mut(&poll_key).ok_or(PollError::NotFound {
            diagnostic_id: "p5.7-poll-not-found",
        })?;
        if poll.phase != PollPhase::Open {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-poll-closed",
            });
        }
        for id in &answer_ids {
            if !poll.vote_counts.contains_key(id) {
                return Err(PollError::Invalid {
                    diagnostic_id: "p5.7-unknown-answer-id",
                });
            }
        }
        let resp_key = (room_id.clone(), response_event_id.clone());
        // Remove prior response from this event id if re-applied.
        if let Some((prev_start, prev_sender, prev_answers)) = self.responses.remove(&resp_key) {
            if let Some(p) = self.polls.get_mut(&(room_id.clone(), prev_start)) {
                for a in prev_answers {
                    if let Some(c) = p.vote_counts.get_mut(&a) {
                        *c = c.saturating_sub(1);
                    }
                }
                // total_responses: only decrement if this was the user's sole tracked
                // response event — approximate by always keeping count of response events.
                let _ = prev_sender;
                p.total_responses = p.total_responses.saturating_sub(1);
            }
        }
        // Drop older responses from same user on this poll (one vote set per user).
        let stale: Vec<_> = self
            .responses
            .iter()
            .filter(|((r, _), (start, s, _))| {
                r == &room_id && start == &start_event_id && s == &sender
            })
            .map(|((r, e), v)| ((r.clone(), e.clone()), v.clone()))
            .collect();
        for (k, (st, _s, prev_answers)) in stale {
            self.responses.remove(&k);
            if let Some(p) = self.polls.get_mut(&(room_id.clone(), st)) {
                for a in prev_answers {
                    if let Some(c) = p.vote_counts.get_mut(&a) {
                        *c = c.saturating_sub(1);
                    }
                }
                p.total_responses = p.total_responses.saturating_sub(1);
            }
        }
        let poll = self.polls.get_mut(&poll_key).expect("poll exists");
        for a in &answer_ids {
            *poll.vote_counts.entry(a.clone()).or_insert(0) += 1;
        }
        poll.total_responses = poll.total_responses.saturating_add(1);
        self.responses
            .insert(resp_key, (start_event_id, sender, answer_ids));
        Ok(())
    }

    /// Close a poll with an end event.
    pub fn close_poll(
        &mut self,
        room_id: &str,
        start_event_id: &str,
        end_event_id: &str,
    ) -> Result<(), PollError> {
        Self::validate_room(room_id)?;
        Self::validate_event_id(start_event_id)?;
        Self::validate_event_id(end_event_id)?;
        let poll = self
            .polls
            .get_mut(&(room_id.to_owned(), start_event_id.to_owned()))
            .ok_or(PollError::NotFound {
                diagnostic_id: "p5.7-poll-not-found",
            })?;
        poll.phase = PollPhase::Closed;
        poll.end_event_id = Some(end_event_id.to_owned());
        Ok(())
    }

    pub fn get(&self, room_id: &str, start_event_id: &str) -> Option<&PollSummary> {
        self.polls
            .get(&(room_id.to_owned(), start_event_id.to_owned()))
    }

    pub fn list_for_room(&self, room_id: &str) -> Vec<&PollSummary> {
        let mut v: Vec<_> = self
            .polls
            .values()
            .filter(|p| p.room_id == room_id)
            .collect();
        v.sort_by(|a, b| b.origin_server_ts.cmp(&a.origin_server_ts));
        v
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.polls.clear();
        self.responses.clear();
    }
}

/// Room state event projection (type + state_key → latest item).
#[derive(Debug, Default)]
pub struct StateEventIndex {
    session_generation: u64,
    /// (room, state_type, state_key) → item
    by_key: HashMap<(RoomId, String, String), TimelineStateItem>,
}

impl StateEventIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_key: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    pub fn upsert(&mut self, item: TimelineStateItem) -> Result<(), PollError> {
        PollIndex::validate_room(&item.room_id)?;
        PollIndex::validate_event_id(&item.event_id)?;
        PollIndex::validate_user(&item.sender)?;
        if item.state_type.is_empty() {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-empty-state-type",
            });
        }
        if let Some(s) = &item.summary {
            PollIndex::forbid_secret_text(s)?;
        }
        let room_count = self
            .by_key
            .keys()
            .filter(|(r, _, _)| r == &item.room_id)
            .count();
        let key = (
            item.room_id.clone(),
            item.state_type.clone(),
            item.state_key.clone(),
        );
        if !self.by_key.contains_key(&key) && room_count >= MAX_STATE_KEYS_PER_ROOM {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-state-cap",
            });
        }
        self.by_key.insert(key, item);
        Ok(())
    }

    pub fn get(
        &self,
        room_id: &str,
        state_type: &str,
        state_key: &str,
    ) -> Option<&TimelineStateItem> {
        self.by_key.get(&(
            room_id.to_owned(),
            state_type.to_owned(),
            state_key.to_owned(),
        ))
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_key.clear();
    }
}

/// Membership change feed (ordered by origin_server_ts).
#[derive(Debug, Default)]
pub struct MembershipEventIndex {
    session_generation: u64,
    by_event: HashMap<(RoomId, EventId), TimelineMembershipItem>,
}

impl MembershipEventIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_event: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_event.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_event.is_empty()
    }

    pub fn upsert(&mut self, item: TimelineMembershipItem) -> Result<(), PollError> {
        PollIndex::validate_room(&item.room_id)?;
        PollIndex::validate_event_id(&item.event_id)?;
        PollIndex::validate_user(&item.sender)?;
        PollIndex::validate_user(&item.target_user_id)?;
        PollIndex::forbid_secret_text(&item.summary)?;
        if item.summary.is_empty() {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-empty-membership-summary",
            });
        }
        let key = (item.room_id.clone(), item.event_id.clone());
        if !self.by_event.contains_key(&key) && self.by_event.len() >= MAX_MEMBERSHIP_EVENTS {
            return Err(PollError::Invalid {
                diagnostic_id: "p5.7-membership-cap",
            });
        }
        self.by_event.insert(key, item);
        Ok(())
    }

    pub fn list_for_room(&self, room_id: &str) -> Vec<&TimelineMembershipItem> {
        let mut v: Vec<_> = self
            .by_event
            .values()
            .filter(|m| m.room_id == room_id)
            .collect();
        v.sort_by(|a, b| a.origin_server_ts.cmp(&b.origin_server_ts));
        v
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_event.clear();
    }
}
