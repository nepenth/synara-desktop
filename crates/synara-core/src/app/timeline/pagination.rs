//! Timeline pagination state machine (P5.3 harness foundation).
//!
//! Tracks backwards/forwards page requests for one timeline stream without
//! calling SDK `paginate_*` yet. Host adapters will drive real network later.
//! No event plaintext, tokens, or dual-backend.

use serde::{Deserialize, Serialize};

use super::error::TimelineError;
use super::registry::TimelineKey;

/// Direction of a pagination request (SDK: paginate_backwards / forwards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaginationDirection {
    /// Older events (toward room start).
    Backwards,
    /// Newer events (toward live end) — rare for live timelines.
    Forwards,
}

impl PaginationDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backwards => "backwards",
            Self::Forwards => "forwards",
        }
    }
}

/// Phase of pagination for one direction on a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaginationPhase {
    /// Ready to accept a request.
    Idle,
    /// Request in flight (host must not double-start same direction).
    InFlight,
    /// No more history in this direction (exhausted).
    Exhausted,
    /// Last request failed; may retry after clear_failure / host policy.
    Failed,
}

impl PaginationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::InFlight => "in_flight",
            Self::Exhausted => "exhausted",
            Self::Failed => "failed",
        }
    }

    pub fn can_start(self) -> bool {
        matches!(self, Self::Idle | Self::Failed)
    }
}

/// Privacy-safe request to load another page (no tokens / no event dumps).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationRequest {
    pub direction: PaginationDirection,
    /// Soft upper bound on items to fetch; host/SDK may return fewer.
    pub limit: u32,
}

impl PaginationRequest {
    pub fn backwards(limit: u32) -> Self {
        Self {
            direction: PaginationDirection::Backwards,
            limit,
        }
    }

    pub fn forwards(limit: u32) -> Self {
        Self {
            direction: PaginationDirection::Forwards,
            limit,
        }
    }

    pub fn validate(&self) -> Result<(), TimelineError> {
        if self.limit == 0 || self.limit > 100 {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-invalid-page-limit",
            });
        }
        Ok(())
    }
}

/// Outcome of a completed page load (counts only — items arrive via P5.2 deltas).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationOutcome {
    pub direction: PaginationDirection,
    /// How many items the host applied (0 is valid; may mean exhausted).
    pub items_applied: u32,
    /// True when the host/SDK reports no further history in this direction.
    pub exhausted: bool,
}

/// Per-direction pagination status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionStatus {
    pub phase: PaginationPhase,
    pub last_limit: Option<u32>,
    pub pages_completed: u32,
    pub items_loaded: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_diagnostic_id: Option<&'static str>,
}

impl Default for DirectionStatus {
    fn default() -> Self {
        Self {
            phase: PaginationPhase::Idle,
            last_limit: None,
            pages_completed: 0,
            items_loaded: 0,
            failure_diagnostic_id: None,
        }
    }
}

/// Pagination controller for one timeline stream (keyed + generation-stamped).
#[derive(Debug, Clone, PartialEq)]
pub struct TimelinePagination {
    key: TimelineKey,
    session_generation: u64,
    backwards: DirectionStatus,
    forwards: DirectionStatus,
}

impl TimelinePagination {
    pub fn new(key: TimelineKey, session_generation: u64) -> Self {
        Self {
            key,
            session_generation,
            backwards: DirectionStatus::default(),
            forwards: DirectionStatus::default(),
        }
    }

    pub fn key(&self) -> &TimelineKey {
        &self.key
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn status(&self, direction: PaginationDirection) -> &DirectionStatus {
        match direction {
            PaginationDirection::Backwards => &self.backwards,
            PaginationDirection::Forwards => &self.forwards,
        }
    }

    fn status_mut(&mut self, direction: PaginationDirection) -> &mut DirectionStatus {
        match direction {
            PaginationDirection::Backwards => &mut self.backwards,
            PaginationDirection::Forwards => &mut self.forwards,
        }
    }

    /// Begin a page request. Rejects invalid limit, in-flight, or exhausted.
    pub fn begin(&mut self, request: PaginationRequest) -> Result<(), TimelineError> {
        request.validate()?;
        let st = self.status_mut(request.direction);
        if st.phase == PaginationPhase::InFlight {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-pagination-already-in-flight",
            });
        }
        if st.phase == PaginationPhase::Exhausted {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-pagination-exhausted",
            });
        }
        if !st.phase.can_start() {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-pagination-not-startable",
            });
        }
        st.phase = PaginationPhase::InFlight;
        st.last_limit = Some(request.limit);
        st.failure_diagnostic_id = None;
        Ok(())
    }

    /// Complete a successful page (host applied deltas separately via P5.2).
    pub fn complete(&mut self, outcome: PaginationOutcome) -> Result<(), TimelineError> {
        let st = self.status_mut(outcome.direction);
        if st.phase != PaginationPhase::InFlight {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-complete-not-in-flight",
            });
        }
        st.pages_completed = st.pages_completed.saturating_add(1);
        st.items_loaded = st.items_loaded.saturating_add(outcome.items_applied);
        st.failure_diagnostic_id = None;
        st.phase = if outcome.exhausted {
            PaginationPhase::Exhausted
        } else {
            PaginationPhase::Idle
        };
        Ok(())
    }

    /// Mark in-flight request failed (privacy-safe diagnostic id only).
    pub fn fail(
        &mut self,
        direction: PaginationDirection,
        diagnostic_id: &'static str,
    ) -> Result<(), TimelineError> {
        let st = self.status_mut(direction);
        if st.phase != PaginationPhase::InFlight {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-fail-not-in-flight",
            });
        }
        st.phase = PaginationPhase::Failed;
        st.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    /// Clear Failed → Idle so host may retry.
    pub fn clear_failure(&mut self, direction: PaginationDirection) -> Result<(), TimelineError> {
        let st = self.status_mut(direction);
        if st.phase != PaginationPhase::Failed {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.3-clear-failure-not-failed",
            });
        }
        st.phase = PaginationPhase::Idle;
        st.failure_diagnostic_id = None;
        Ok(())
    }

    /// Retire on generation bump (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        for st in [&mut self.backwards, &mut self.forwards] {
            if st.phase == PaginationPhase::InFlight {
                st.phase = PaginationPhase::Failed;
                st.failure_diagnostic_id = Some("p5.3-stale-generation-cancelled");
            }
        }
    }

    /// True if any direction is loading.
    pub fn any_in_flight(&self) -> bool {
        self.backwards.phase == PaginationPhase::InFlight
            || self.forwards.phase == PaginationPhase::InFlight
    }
}
