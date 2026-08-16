//! Timeline registry and lifecycle (P5.1 harness foundation).
//!
//! Tracks per-room timeline owners stamped with supervisor session generation.
//! Does **not** construct SDK `Timeline` objects yet (P5.2 mapping). No dual-backend.

use std::collections::HashMap;

use crate::dto::RoomId;

use super::error::TimelineError;

/// Opaque registry key for one room timeline subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimelineKey {
    pub room_id: RoomId,
    /// Optional thread root event id; `None` = main room timeline.
    pub thread_root: Option<String>,
}

impl TimelineKey {
    pub fn main(room_id: impl Into<String>) -> Result<Self, TimelineError> {
        let room_id = room_id.into().trim().to_owned();
        if room_id.is_empty() || !room_id.starts_with('!') {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.1-invalid-room-id",
            });
        }
        Ok(Self {
            room_id,
            thread_root: None,
        })
    }

    pub fn thread(
        room_id: impl Into<String>,
        thread_root: impl Into<String>,
    ) -> Result<Self, TimelineError> {
        let mut key = Self::main(room_id)?;
        let root = thread_root.into().trim().to_owned();
        if root.is_empty() {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.1-invalid-thread-root",
            });
        }
        key.thread_root = Some(root);
        Ok(key)
    }
}

/// Lifecycle of a registered timeline handle (no SDK object).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineLifecycle {
    /// Entry reserved; not yet marked live.
    Opening,
    /// Ready for snapshot/diff consumers (P5.2+).
    Live,
    /// Explicitly closed; may be reopened.
    Closed,
    /// Terminal failure for this generation.
    Failed,
}

impl TimelineLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opening => "opening",
            Self::Live => "live",
            Self::Closed => "closed",
            Self::Failed => "failed",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Opening | Self::Live)
    }
}

/// Privacy-safe registry entry (no tokens, no event plaintext dumps).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    pub key: TimelineKey,
    pub session_generation: u64,
    pub lifecycle: TimelineLifecycle,
    pub failure_diagnostic_id: Option<&'static str>,
}

/// Per-session-generation registry of room timelines.
#[derive(Debug, Default)]
pub struct TimelineRegistry {
    session_generation: u64,
    entries: HashMap<(RoomId, Option<String>), TimelineEntry>,
}

impl TimelineRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            entries: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| e.lifecycle.is_active())
            .count()
    }

    fn map_key(key: &TimelineKey) -> (RoomId, Option<String>) {
        (key.room_id.clone(), key.thread_root.clone())
    }

    /// Begin opening a timeline for `key` in the current generation.
    pub fn open(&mut self, key: TimelineKey) -> Result<&TimelineEntry, TimelineError> {
        let mk = Self::map_key(&key);
        if let Some(existing) = self.entries.get(&mk) {
            if existing.lifecycle.is_active() {
                return Err(TimelineError::AlreadyOpen {
                    diagnostic_id: "p5.1-timeline-already-open",
                });
            }
        }
        let entry = TimelineEntry {
            key: key.clone(),
            session_generation: self.session_generation,
            lifecycle: TimelineLifecycle::Opening,
            failure_diagnostic_id: None,
        };
        self.entries.insert(mk.clone(), entry);
        Ok(self.entries.get(&mk).expect("just inserted"))
    }

    /// Mark opening timeline as live (SDK attach succeeded — host calls this).
    pub fn mark_live(&mut self, key: &TimelineKey) -> Result<&TimelineEntry, TimelineError> {
        let mk = Self::map_key(key);
        let entry = self.entries.get_mut(&mk).ok_or(TimelineError::NotFound {
            diagnostic_id: "p5.1-timeline-not-found",
        })?;
        if entry.session_generation != self.session_generation {
            return Err(TimelineError::StaleGeneration {
                diagnostic_id: "p5.1-stale-timeline-generation",
                expected: self.session_generation,
                observed: entry.session_generation,
            });
        }
        if entry.lifecycle != TimelineLifecycle::Opening
            && entry.lifecycle != TimelineLifecycle::Live
        {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.1-mark-live-invalid-state",
            });
        }
        entry.lifecycle = TimelineLifecycle::Live;
        entry.failure_diagnostic_id = None;
        Ok(entry)
    }

    /// Mark timeline failed (privacy-safe diagnostic only).
    pub fn mark_failed(
        &mut self,
        key: &TimelineKey,
        diagnostic_id: &'static str,
    ) -> Result<&TimelineEntry, TimelineError> {
        let mk = Self::map_key(key);
        let entry = self.entries.get_mut(&mk).ok_or(TimelineError::NotFound {
            diagnostic_id: "p5.1-timeline-not-found",
        })?;
        entry.lifecycle = TimelineLifecycle::Failed;
        entry.failure_diagnostic_id = Some(diagnostic_id);
        Ok(entry)
    }

    /// Close a timeline (detach consumers; entry retained as Closed).
    pub fn close(&mut self, key: &TimelineKey) -> Result<(), TimelineError> {
        let mk = Self::map_key(key);
        let entry = self.entries.get_mut(&mk).ok_or(TimelineError::NotFound {
            diagnostic_id: "p5.1-timeline-not-found",
        })?;
        entry.lifecycle = TimelineLifecycle::Closed;
        entry.failure_diagnostic_id = None;
        Ok(())
    }

    /// Drop closed/failed entries; active ones closed first.
    pub fn dispose(&mut self, key: &TimelineKey) -> Result<(), TimelineError> {
        let mk = Self::map_key(key);
        if !self.entries.contains_key(&mk) {
            return Err(TimelineError::NotFound {
                diagnostic_id: "p5.1-timeline-not-found",
            });
        }
        self.entries.remove(&mk);
        Ok(())
    }

    pub fn get(&self, key: &TimelineKey) -> Option<&TimelineEntry> {
        self.entries.get(&Self::map_key(key))
    }

    /// Bump session generation and close all active timelines (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        for entry in self.entries.values_mut() {
            if entry.lifecycle.is_active() {
                entry.lifecycle = TimelineLifecycle::Closed;
            }
            entry.session_generation = new_generation;
        }
    }

    /// Remove all entries (hard reset after wipe).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn list(&self) -> Vec<&TimelineEntry> {
        let mut v: Vec<_> = self.entries.values().collect();
        v.sort_by(|a, b| {
            a.key
                .room_id
                .cmp(&b.key.room_id)
                .then_with(|| a.key.thread_root.cmp(&b.key.thread_root))
        });
        v
    }
}
