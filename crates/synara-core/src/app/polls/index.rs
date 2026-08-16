//! Ordered poll and room state/membership projection indexes (P5.7 harness).

use std::collections::BTreeMap;

use crate::dto::{EventId, RoomId};

use super::error::ProjectionError;
use super::model::{PollProjection, StateProjectionRow};

type ProjectionKey = (RoomId, EventId);

fn validate_key(room_id: &str, event_id: &str) -> Result<(), ProjectionError> {
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(ProjectionError::Invalid {
            diagnostic_id: "p5.7-invalid-room-id",
        });
    }
    if event_id.is_empty() || !event_id.starts_with('$') {
        return Err(ProjectionError::Invalid {
            diagnostic_id: "p5.7-invalid-event-id",
        });
    }
    Ok(())
}

/// Session-generation-stamped poll summary index.
///
/// The index intentionally has no `Debug`/`Display` implementation because its
/// rows contain poll question plaintext.
#[derive(Default)]
pub struct PollIndex {
    session_generation: u64,
    rows: BTreeMap<ProjectionKey, PollProjection>,
}

impl PollIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            rows: BTreeMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Insert a poll or replace the row with the same room/event identity.
    pub fn upsert(
        &mut self,
        row: PollProjection,
    ) -> Result<Option<PollProjection>, ProjectionError> {
        validate_key(&row.room_id, &row.poll_event_id)?;
        if row.question.is_empty() {
            return Err(ProjectionError::Invalid {
                diagnostic_id: "p5.7-empty-poll-question",
            });
        }
        if row
            .response_counts
            .keys()
            .any(|answer_id| answer_id.is_empty())
        {
            return Err(ProjectionError::Invalid {
                diagnostic_id: "p5.7-empty-answer-id",
            });
        }
        let key = (row.room_id.clone(), row.poll_event_id.clone());
        Ok(self.rows.insert(key, row))
    }

    pub fn get(&self, room_id: &str, poll_event_id: &str) -> Option<&PollProjection> {
        self.rows
            .get(&(room_id.to_owned(), poll_event_id.to_owned()))
    }

    /// Remove one poll by room and poll event id.
    pub fn remove(&mut self, room_id: &str, poll_event_id: &str) -> Option<PollProjection> {
        self.rows
            .remove(&(room_id.to_owned(), poll_event_id.to_owned()))
    }

    /// Polls in one room, ordered by poll event id.
    pub fn list_room(&self, room_id: &str) -> Vec<&PollProjection> {
        self.rows
            .range((room_id.to_owned(), String::new())..)
            .take_while(|((row_room_id, _), _)| row_room_id == room_id)
            .map(|(_, row)| row)
            .collect()
    }

    pub fn clear_room(&mut self, room_id: &str) {
        self.rows
            .retain(|(row_room_id, _), _| row_room_id != room_id);
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }

    /// Bump generation and wipe on logout/account switch.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}

/// Session-generation-stamped simple room state/membership summary index.
///
/// The index intentionally has no `Debug`/`Display` implementation because its
/// rows may contain event plaintext in `summary`.
#[derive(Default)]
pub struct StateProjectionIndex {
    session_generation: u64,
    rows: BTreeMap<ProjectionKey, StateProjectionRow>,
}

impl StateProjectionIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            rows: BTreeMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Insert a state row or replace the row with the same room/event identity.
    pub fn upsert(
        &mut self,
        row: StateProjectionRow,
    ) -> Result<Option<StateProjectionRow>, ProjectionError> {
        validate_key(&row.room_id, &row.event_id)?;
        if row
            .target_user_localpart
            .as_ref()
            .is_some_and(|localpart| localpart.is_empty() || localpart.contains(['@', ':']))
        {
            return Err(ProjectionError::Invalid {
                diagnostic_id: "p5.7-invalid-user-localpart",
            });
        }
        let key = (row.room_id.clone(), row.event_id.clone());
        Ok(self.rows.insert(key, row))
    }

    pub fn get(&self, room_id: &str, event_id: &str) -> Option<&StateProjectionRow> {
        self.rows.get(&(room_id.to_owned(), event_id.to_owned()))
    }

    /// Remove one state row by room and event id.
    pub fn remove(&mut self, room_id: &str, event_id: &str) -> Option<StateProjectionRow> {
        self.rows.remove(&(room_id.to_owned(), event_id.to_owned()))
    }

    /// State rows in one room, ordered by event id.
    pub fn list_room(&self, room_id: &str) -> Vec<&StateProjectionRow> {
        self.rows
            .range((room_id.to_owned(), String::new())..)
            .take_while(|((row_room_id, _), _)| row_room_id == room_id)
            .map(|(_, row)| row)
            .collect()
    }

    pub fn clear_room(&mut self, room_id: &str) {
        self.rows
            .retain(|(row_room_id, _), _| row_room_id != room_id);
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }

    /// Bump generation and wipe on logout/account switch.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}
