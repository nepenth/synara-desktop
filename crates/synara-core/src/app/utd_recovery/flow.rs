//! UTD retry / encrypted-history recovery flow (P8.7 harness foundation).
//!
//! Coordinates bulk retry and history-recovery UX. **No megolm keys, no event
//! bodies, no dual-backend.** Complements P5.10 per-event `UtdIndex` with a
//! room/session-level recovery session state machine.

use std::collections::HashMap;

use crate::dto::{EventId, RoomId};

use super::error::UtdRecoveryError;

/// Soft caps.
pub const MAX_ROOM_SESSIONS: usize = 512;
pub const MAX_EVENT_IDS_PER_BATCH: usize = 256;

/// Kind of recovery work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtdRecoveryKind {
    /// Retry decrypt for known UTD events (key request / backup hit).
    RetryDecrypt,
    /// Recover encrypted history after backup/restore or key import.
    EncryptedHistoryRecovery,
}

impl UtdRecoveryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RetryDecrypt => "retry_decrypt",
            Self::EncryptedHistoryRecovery => "encrypted_history_recovery",
        }
    }
}

/// Phase of a recovery session for one room (or global batch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtdRecoveryPhase {
    Idle,
    Queued,
    InFlight,
    PartialSuccess,
    Succeeded,
    Failed,
    Cancelled,
}

impl UtdRecoveryPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Queued => "queued",
            Self::InFlight => "in_flight",
            Self::PartialSuccess => "partial_success",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::InFlight)
    }
}

/// One room-scoped recovery session (counts only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtdRecoverySession {
    pub room_id: RoomId,
    pub kind: UtdRecoveryKind,
    pub phase: UtdRecoveryPhase,
    /// Event ids queued for retry (ids only).
    pub pending_event_ids: Vec<EventId>,
    pub recovered_count: u32,
    pub still_utd_count: u32,
    pub failure_diagnostic_id: Option<&'static str>,
    pub op_id: u64,
}

/// Session-generation-stamped UTD recovery coordinator.
#[derive(Debug, Default)]
pub struct UtdRecoveryCoordinator {
    session_generation: u64,
    by_room: HashMap<RoomId, UtdRecoverySession>,
    next_op_id: u64,
}

impl UtdRecoveryCoordinator {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_room: HashMap::new(),
            next_op_id: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_room.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_room.is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.by_room
            .values()
            .filter(|s| s.phase.is_active())
            .count()
    }

    pub fn get(&self, room_id: &str) -> Option<&UtdRecoverySession> {
        self.by_room.get(room_id)
    }

    /// Begin recovery for a room with optional pending event ids.
    pub fn begin(
        &mut self,
        room_id: impl Into<String>,
        kind: UtdRecoveryKind,
        pending_event_ids: Vec<EventId>,
    ) -> Result<u64, UtdRecoveryError> {
        let room_id = room_id.into();
        validate_room(&room_id)?;
        if pending_event_ids.len() > MAX_EVENT_IDS_PER_BATCH {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-event-batch-cap",
            });
        }
        for eid in &pending_event_ids {
            validate_event(eid)?;
        }
        if let Some(existing) = self.by_room.get(&room_id) {
            if existing.phase.is_active() {
                return Err(UtdRecoveryError::Invalid {
                    diagnostic_id: "p8.7-room-already-active",
                });
            }
        } else if self.by_room.len() >= MAX_ROOM_SESSIONS {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-room-session-cap",
            });
        }
        self.next_op_id = self.next_op_id.saturating_add(1);
        let op_id = self.next_op_id;
        self.by_room.insert(
            room_id.clone(),
            UtdRecoverySession {
                room_id,
                kind,
                phase: UtdRecoveryPhase::Queued,
                pending_event_ids,
                recovered_count: 0,
                still_utd_count: 0,
                failure_diagnostic_id: None,
                op_id,
            },
        );
        Ok(op_id)
    }

    pub fn mark_in_flight(&mut self, room_id: &str, op_id: u64) -> Result<(), UtdRecoveryError> {
        let s = self.session_mut(room_id, op_id)?;
        if s.phase != UtdRecoveryPhase::Queued && s.phase != UtdRecoveryPhase::InFlight {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-invalid-phase",
            });
        }
        s.phase = UtdRecoveryPhase::InFlight;
        Ok(())
    }

    /// Host reports partial progress (some events recovered).
    pub fn report_progress(
        &mut self,
        room_id: &str,
        op_id: u64,
        newly_recovered: u32,
        still_utd: u32,
    ) -> Result<(), UtdRecoveryError> {
        let s = self.session_mut(room_id, op_id)?;
        if !s.phase.is_active() {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-invalid-phase",
            });
        }
        s.recovered_count = s.recovered_count.saturating_add(newly_recovered);
        s.still_utd_count = still_utd;
        if newly_recovered > 0 && still_utd > 0 {
            s.phase = UtdRecoveryPhase::PartialSuccess;
            // stay "active" for further work — re-queue
            s.phase = UtdRecoveryPhase::InFlight;
        }
        Ok(())
    }

    pub fn succeed(
        &mut self,
        room_id: &str,
        op_id: u64,
        recovered: u32,
        still_utd: u32,
    ) -> Result<(), UtdRecoveryError> {
        let s = self.session_mut(room_id, op_id)?;
        if !s.phase.is_active() && s.phase != UtdRecoveryPhase::PartialSuccess {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-invalid-phase",
            });
        }
        s.recovered_count = recovered;
        s.still_utd_count = still_utd;
        s.pending_event_ids.clear();
        s.phase = if still_utd > 0 {
            UtdRecoveryPhase::PartialSuccess
        } else {
            UtdRecoveryPhase::Succeeded
        };
        Ok(())
    }

    pub fn fail(
        &mut self,
        room_id: &str,
        op_id: u64,
        diagnostic_id: &'static str,
    ) -> Result<(), UtdRecoveryError> {
        validate_diagnostic(diagnostic_id)?;
        let s = self.session_mut(room_id, op_id)?;
        if !s.phase.is_active() {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-invalid-phase",
            });
        }
        s.phase = UtdRecoveryPhase::Failed;
        s.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    pub fn cancel(&mut self, room_id: &str, op_id: u64) -> Result<(), UtdRecoveryError> {
        let s = self.session_mut(room_id, op_id)?;
        if !s.phase.is_active() {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-invalid-phase",
            });
        }
        s.phase = UtdRecoveryPhase::Cancelled;
        Ok(())
    }

    pub fn clear_room(&mut self, room_id: &str) -> bool {
        self.by_room.remove(room_id).is_some()
    }

    pub fn list_active(&self) -> Vec<&UtdRecoverySession> {
        let mut v: Vec<_> = self
            .by_room
            .values()
            .filter(|s| s.phase.is_active() || s.phase == UtdRecoveryPhase::PartialSuccess)
            .collect();
        v.sort_by(|a, b| a.room_id.cmp(&b.room_id));
        v
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_room.clear();
        self.next_op_id = 0;
    }

    fn session_mut(
        &mut self,
        room_id: &str,
        op_id: u64,
    ) -> Result<&mut UtdRecoverySession, UtdRecoveryError> {
        let s = self
            .by_room
            .get_mut(room_id)
            .ok_or(UtdRecoveryError::NotFound {
                diagnostic_id: "p8.7-room-not-found",
            })?;
        if s.op_id != op_id {
            return Err(UtdRecoveryError::Invalid {
                diagnostic_id: "p8.7-stale-op-id",
            });
        }
        Ok(s)
    }
}

fn validate_room(room_id: &str) -> Result<(), UtdRecoveryError> {
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(UtdRecoveryError::Invalid {
            diagnostic_id: "p8.7-invalid-room-id",
        });
    }
    Ok(())
}

fn validate_event(event_id: &str) -> Result<(), UtdRecoveryError> {
    if event_id.is_empty() || !event_id.starts_with('$') {
        return Err(UtdRecoveryError::Invalid {
            diagnostic_id: "p8.7-invalid-event-id",
        });
    }
    Ok(())
}

fn validate_diagnostic(diagnostic_id: &'static str) -> Result<(), UtdRecoveryError> {
    if diagnostic_id.is_empty() {
        return Err(UtdRecoveryError::Invalid {
            diagnostic_id: "p8.7-empty-diagnostic",
        });
    }
    let lower = diagnostic_id.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("session_key")
        || lower.contains("password")
        || lower.contains("private")
    {
        return Err(UtdRecoveryError::Invalid {
            diagnostic_id: "p8.7-forbidden-diagnostic",
        });
    }
    Ok(())
}
