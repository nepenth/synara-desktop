//! Pure notification destination resolution and registry (P9.4).

use std::collections::HashMap;

use crate::matrix::dto::{EventId, NotificationCandidate, NotificationCandidateId, RoomId};

use super::error::NotificationRoutingError;

/// Destination specificity selected for a notification candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationRouteKind {
    Room,
    Event,
    Thread,
}

/// Validated internal destination for a notification candidate.
///
/// This contract contains identifiers only. It never carries notification
/// title/body text, event content, media, credentials, or recovery material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRoute {
    pub room_id: RoomId,
    pub event_id: Option<EventId>,
    pub thread_root_id: Option<EventId>,
    pub kind: NotificationRouteKind,
}

/// Session-generation-stamped registry of the last resolved route per
/// candidate key.
#[derive(Debug, Default)]
pub struct NotificationRouter {
    session_generation: u64,
    by_candidate: HashMap<NotificationCandidateId, NotificationRoute>,
}

impl NotificationRouter {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_candidate: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_candidate.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_candidate.is_empty()
    }

    /// Resolve candidate fields and retain the result as the candidate's last
    /// route.
    ///
    /// A thread root takes precedence over an event anchor; an event anchor
    /// takes precedence over the room-only destination.
    pub fn resolve(
        &mut self,
        candidate_key: NotificationCandidateId,
        room_id: RoomId,
        event_id: Option<EventId>,
        thread_root_id: Option<EventId>,
    ) -> Result<NotificationRoute, NotificationRoutingError> {
        validate_candidate_key(&candidate_key)?;
        validate_room_id(&room_id)?;
        if let Some(event_id) = event_id.as_deref() {
            validate_event_id(event_id, "p9.4-invalid-event-id")?;
        }
        if let Some(thread_root_id) = thread_root_id.as_deref() {
            validate_event_id(thread_root_id, "p9.4-invalid-thread-root-id")?;
        }

        let kind = if thread_root_id.is_some() {
            NotificationRouteKind::Thread
        } else if event_id.is_some() {
            NotificationRouteKind::Event
        } else {
            NotificationRouteKind::Room
        };
        let route = NotificationRoute {
            room_id,
            event_id,
            thread_root_id,
            kind,
        };
        self.by_candidate.insert(candidate_key, route.clone());
        Ok(route)
    }

    /// Resolve the routing fields already present on a P7.1 candidate, plus
    /// the optional thread relation supplied by the caller.
    pub fn resolve_candidate(
        &mut self,
        candidate: &NotificationCandidate,
        thread_root_id: Option<EventId>,
    ) -> Result<NotificationRoute, NotificationRoutingError> {
        self.resolve(
            candidate.candidate_id.clone(),
            candidate.room_id.clone(),
            candidate.event_id.clone(),
            thread_root_id,
        )
    }

    pub fn last_route(&self, candidate_key: &str) -> Option<&NotificationRoute> {
        self.by_candidate.get(candidate_key)
    }

    pub fn clear(&mut self) {
        self.by_candidate.clear();
    }

    /// Bump generation and remove routes from the retired session.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}

fn validate_candidate_key(candidate_key: &str) -> Result<(), NotificationRoutingError> {
    if candidate_key.is_empty() {
        return Err(NotificationRoutingError::invalid(
            "p9.4-empty-candidate-key",
        ));
    }
    Ok(())
}

fn validate_room_id(room_id: &str) -> Result<(), NotificationRoutingError> {
    let Some((localpart, server_name)) = room_id
        .strip_prefix('!')
        .and_then(|room_id| room_id.split_once(':'))
    else {
        return Err(NotificationRoutingError::invalid("p9.4-invalid-room-id"));
    };
    if localpart.is_empty() || server_name.is_empty() || room_id.chars().any(char::is_whitespace) {
        return Err(NotificationRoutingError::invalid("p9.4-invalid-room-id"));
    }
    Ok(())
}

fn validate_event_id(
    event_id: &str,
    diagnostic_id: &'static str,
) -> Result<(), NotificationRoutingError> {
    if event_id.len() < 2 || !event_id.starts_with('$') || event_id.chars().any(char::is_whitespace)
    {
        return Err(NotificationRoutingError::invalid(diagnostic_id));
    }
    Ok(())
}
