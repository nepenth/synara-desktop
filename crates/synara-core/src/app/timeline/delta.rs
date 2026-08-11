//! Product timeline deltas (no SDK `VectorDiff` / `TimelineItem` on the wire).
//!
//! Mirrors the semantic shape of `eyeball_im::VectorDiff` so host projections
//! can apply ordered ops deterministically. Values are Synara [`TimelineItem`]
//! DTOs only — never raw matrix-sdk types. Mapping from SDK diffs is host-side
//! (still harness until cutover).

use serde::{Deserialize, Serialize};

use crate::dto::TimelineItem;

/// One ordered mutation against a timeline projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TimelineDeltaOp {
    /// Replace the entire ordered item list (initial snapshot or hard reset).
    Reset { items: Vec<TimelineItem> },
    /// Append items at the end (typically older history when reverse-paginating
    /// is modeled as prepend — host chooses orientation; foundation is ordered).
    Append { items: Vec<TimelineItem> },
    /// Clear all items.
    Clear,
    /// Push one item at the front.
    PushFront { item: TimelineItem },
    /// Push one item at the back.
    PushBack { item: TimelineItem },
    /// Insert at index (0..=len).
    Insert { index: usize, item: TimelineItem },
    /// Replace item at index.
    Set { index: usize, item: TimelineItem },
    /// Remove item at index.
    Remove { index: usize },
    /// Keep only the first `len` items.
    Truncate { len: usize },
    /// Move item from `from` to `to`.
    Move { from: usize, to: usize },
}

impl TimelineDeltaOp {
    pub fn op_name(&self) -> &'static str {
        match self {
            Self::Reset { .. } => "reset",
            Self::Append { .. } => "append",
            Self::Clear => "clear",
            Self::PushFront { .. } => "push_front",
            Self::PushBack { .. } => "push_back",
            Self::Insert { .. } => "insert",
            Self::Set { .. } => "set",
            Self::Remove { .. } => "remove",
            Self::Truncate { .. } => "truncate",
            Self::Move { .. } => "move",
        }
    }
}

/// Ordered batch of ops with stream sequence + session generation.
///
/// `sequence` is monotonic per generation per timeline key. Gap ⇒ resync
/// (full `Reset` snapshot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineDeltaBatch {
    pub session_generation: u64,
    pub sequence: u64,
    pub ops: Vec<TimelineDeltaOp>,
}

/// Full snapshot envelope for one timeline stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSnapshot {
    pub session_generation: u64,
    pub sequence: u64,
    pub items: Vec<TimelineItem>,
}

impl TimelineSnapshot {
    pub fn empty(session_generation: u64) -> Self {
        Self {
            session_generation,
            sequence: 0,
            items: Vec::new(),
        }
    }

    pub fn into_reset_batch(self) -> TimelineDeltaBatch {
        TimelineDeltaBatch {
            session_generation: self.session_generation,
            sequence: self.sequence,
            ops: vec![TimelineDeltaOp::Reset { items: self.items }],
        }
    }
}
