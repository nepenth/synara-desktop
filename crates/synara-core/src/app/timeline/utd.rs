//! UTD / decryption update propagation (P5.10 harness foundation).
//!
//! Tracks unable-to-decrypt (encrypted unavailable) timeline items and
//! subsequent decrypt-success / permanent-failure updates. **No megolm
//! session keys, no event plaintext bodies, no dual-backend.** Host maps
//! SDK crypto events → this index; P5.2 deltas replace timeline rows.

use std::collections::HashMap;

use crate::dto::{EventId, RoomId, TimelineEncryptedUnavailableItem, TimelineItemId};

use super::error::TimelineError;

/// Soft cap on tracked UTD entries per session (UI / memory safety).
pub const MAX_UTD_ENTRIES: usize = 2_048;

/// Why decryption is unavailable (privacy-safe codes only — not raw SDK dumps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtdReasonCode {
    /// Keys not yet received; retry may succeed.
    MissingKeys,
    /// Historical message before this device joined / no key backup hit.
    Historical,
    /// Withheld by sender policy.
    Withheld,
    /// Megolm session unknown / permanently unrecoverable.
    UnknownSession,
    /// Other / unclassified (still no secrets in reason string).
    Other,
}

impl UtdReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingKeys => "missing_keys",
            Self::Historical => "historical",
            Self::Withheld => "withheld",
            Self::UnknownSession => "unknown_session",
            Self::Other => "other",
        }
    }

    pub fn is_retryable(self) -> bool {
        matches!(self, Self::MissingKeys | Self::Historical | Self::Other)
    }
}

/// Lifecycle of one encrypted event in the UTD index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtdPhase {
    /// Currently shown as encrypted/unavailable in the timeline.
    UnableToDecrypt,
    /// Host requested a decrypt/retry (in flight).
    RetryPending,
    /// Successfully decrypted — timeline should replace via P5.2 delta.
    Decrypted,
    /// Permanently failed; stop auto-retry.
    PermanentFailure,
}

impl UtdPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnableToDecrypt => "unable_to_decrypt",
            Self::RetryPending => "retry_pending",
            Self::Decrypted => "decrypted",
            Self::PermanentFailure => "permanent_failure",
        }
    }

    pub fn is_active_utd(self) -> bool {
        matches!(self, Self::UnableToDecrypt | Self::RetryPending)
    }
}

/// One tracked UTD / decrypt state (ids + codes only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtdEntry {
    pub room_id: RoomId,
    pub event_id: EventId,
    pub item_id: TimelineItemId,
    pub reason: UtdReasonCode,
    pub phase: UtdPhase,
    pub retry_count: u32,
    /// Optional host diagnostic id (must not contain secrets).
    pub failure_diagnostic_id: Option<&'static str>,
}

/// Update emitted when decrypt state changes (host applies P5.2 ops).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtdUpdate {
    /// New or refreshed UTD placeholder.
    MarkedUnavailable {
        room_id: RoomId,
        event_id: EventId,
        item_id: TimelineItemId,
        reason: UtdReasonCode,
    },
    /// Decrypt succeeded — replace encrypted row with clear content elsewhere.
    Decrypted {
        room_id: RoomId,
        event_id: EventId,
        item_id: TimelineItemId,
    },
    /// Permanent failure after retries.
    PermanentFailure {
        room_id: RoomId,
        event_id: EventId,
        item_id: TimelineItemId,
        diagnostic_id: &'static str,
    },
}

impl UtdUpdate {
    pub fn event_id(&self) -> &str {
        match self {
            Self::MarkedUnavailable { event_id, .. }
            | Self::Decrypted { event_id, .. }
            | Self::PermanentFailure { event_id, .. } => event_id.as_str(),
        }
    }
}

/// Session-generation-stamped UTD index.
#[derive(Debug, Default)]
pub struct UtdIndex {
    session_generation: u64,
    /// Keyed by (room_id, event_id).
    by_event: HashMap<(RoomId, EventId), UtdEntry>,
}

impl UtdIndex {
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

    /// Count entries still showing as UTD / retry pending.
    pub fn active_utd_count(&self) -> usize {
        self.by_event
            .values()
            .filter(|e| e.phase.is_active_utd())
            .count()
    }

    pub fn get(&self, room_id: &str, event_id: &str) -> Option<&UtdEntry> {
        self.by_event
            .get(&(room_id.to_owned(), event_id.to_owned()))
    }

    fn validate_ids(room_id: &str, event_id: &str, item_id: &str) -> Result<(), TimelineError> {
        if room_id.is_empty() || !room_id.starts_with('!') {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-invalid-room-id",
            });
        }
        if event_id.is_empty() || !event_id.starts_with('$') {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-invalid-event-id",
            });
        }
        if item_id.is_empty() {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-invalid-item-id",
            });
        }
        Ok(())
    }

    fn validate_reason_string(reason: Option<&str>) -> Result<(), TimelineError> {
        if let Some(r) = reason {
            let lower = r.to_ascii_lowercase();
            if lower.contains("access_token")
                || lower.contains("session_key")
                || lower.contains("megolm") && (lower.contains("key") && lower.contains('='))
                || lower.contains("private")
            {
                return Err(TimelineError::Invalid {
                    diagnostic_id: "p5.10-forbidden-reason",
                });
            }
        }
        Ok(())
    }

    /// Upsert from a product encrypted-unavailable DTO (host mapped).
    pub fn mark_unavailable(
        &mut self,
        item: TimelineEncryptedUnavailableItem,
        reason: UtdReasonCode,
    ) -> Result<UtdUpdate, TimelineError> {
        Self::validate_ids(&item.room_id, &item.event_id, &item.item_id)?;
        Self::validate_reason_string(item.reason.as_deref())?;
        let key = (item.room_id.clone(), item.event_id.clone());
        if !self.by_event.contains_key(&key) && self.by_event.len() >= MAX_UTD_ENTRIES {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-utd-cap",
            });
        }
        let entry = UtdEntry {
            room_id: item.room_id.clone(),
            event_id: item.event_id.clone(),
            item_id: item.item_id.clone(),
            reason,
            phase: UtdPhase::UnableToDecrypt,
            retry_count: self.by_event.get(&key).map(|e| e.retry_count).unwrap_or(0),
            failure_diagnostic_id: None,
        };
        self.by_event.insert(key, entry);
        Ok(UtdUpdate::MarkedUnavailable {
            room_id: item.room_id,
            event_id: item.event_id,
            item_id: item.item_id,
            reason,
        })
    }

    /// Begin a decrypt retry for an active UTD entry.
    pub fn begin_retry(&mut self, room_id: &str, event_id: &str) -> Result<(), TimelineError> {
        let e = self
            .by_event
            .get_mut(&(room_id.to_owned(), event_id.to_owned()))
            .ok_or(TimelineError::NotFound {
                diagnostic_id: "p5.10-utd-not-found",
            })?;
        if e.phase == UtdPhase::PermanentFailure {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-permanent-no-retry",
            });
        }
        if e.phase == UtdPhase::Decrypted {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-already-decrypted",
            });
        }
        if e.phase == UtdPhase::RetryPending {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-retry-already-pending",
            });
        }
        if !e.reason.is_retryable() && e.phase == UtdPhase::UnableToDecrypt {
            // Withheld / unknown session: still allow one explicit user retry.
        }
        e.phase = UtdPhase::RetryPending;
        e.retry_count = e.retry_count.saturating_add(1);
        Ok(())
    }

    /// Host reports successful decrypt; entry moves to Decrypted.
    pub fn mark_decrypted(
        &mut self,
        room_id: &str,
        event_id: &str,
    ) -> Result<UtdUpdate, TimelineError> {
        let key = (room_id.to_owned(), event_id.to_owned());
        let e = self.by_event.get_mut(&key).ok_or(TimelineError::NotFound {
            diagnostic_id: "p5.10-utd-not-found",
        })?;
        if e.phase == UtdPhase::Decrypted {
            return Ok(UtdUpdate::Decrypted {
                room_id: e.room_id.clone(),
                event_id: e.event_id.clone(),
                item_id: e.item_id.clone(),
            });
        }
        e.phase = UtdPhase::Decrypted;
        e.failure_diagnostic_id = None;
        Ok(UtdUpdate::Decrypted {
            room_id: e.room_id.clone(),
            event_id: e.event_id.clone(),
            item_id: e.item_id.clone(),
        })
    }

    /// Retry failed but still retryable → back to UnableToDecrypt.
    pub fn retry_failed(
        &mut self,
        room_id: &str,
        event_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<(), TimelineError> {
        validate_diagnostic(diagnostic_id)?;
        let e = self
            .by_event
            .get_mut(&(room_id.to_owned(), event_id.to_owned()))
            .ok_or(TimelineError::NotFound {
                diagnostic_id: "p5.10-utd-not-found",
            })?;
        if e.phase != UtdPhase::RetryPending {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.10-retry-not-pending",
            });
        }
        e.phase = UtdPhase::UnableToDecrypt;
        e.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    /// Mark permanent failure (stop auto-retry).
    pub fn mark_permanent_failure(
        &mut self,
        room_id: &str,
        event_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<UtdUpdate, TimelineError> {
        validate_diagnostic(diagnostic_id)?;
        let e = self
            .by_event
            .get_mut(&(room_id.to_owned(), event_id.to_owned()))
            .ok_or(TimelineError::NotFound {
                diagnostic_id: "p5.10-utd-not-found",
            })?;
        e.phase = UtdPhase::PermanentFailure;
        e.failure_diagnostic_id = Some(diagnostic_id);
        Ok(UtdUpdate::PermanentFailure {
            room_id: e.room_id.clone(),
            event_id: e.event_id.clone(),
            item_id: e.item_id.clone(),
            diagnostic_id,
        })
    }

    /// Active UTD entries for a room (UnableToDecrypt + RetryPending).
    pub fn list_active_for_room(&self, room_id: &str) -> Vec<&UtdEntry> {
        let mut v: Vec<_> = self
            .by_event
            .values()
            .filter(|e| e.room_id == room_id && e.phase.is_active_utd())
            .collect();
        v.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        v
    }

    /// Drop decrypted entries to free cap budget (optional host GC).
    pub fn gc_decrypted(&mut self) -> usize {
        let before = self.by_event.len();
        self.by_event.retain(|_, e| e.phase != UtdPhase::Decrypted);
        before.saturating_sub(self.by_event.len())
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_event.clear();
    }
}

fn validate_diagnostic(diagnostic_id: &'static str) -> Result<(), TimelineError> {
    if diagnostic_id.is_empty() {
        return Err(TimelineError::Invalid {
            diagnostic_id: "p5.10-empty-diagnostic",
        });
    }
    let lower = diagnostic_id.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("session_key")
        || lower.contains("private")
        || lower.contains("password")
    {
        return Err(TimelineError::Invalid {
            diagnostic_id: "p5.10-forbidden-diagnostic",
        });
    }
    Ok(())
}
