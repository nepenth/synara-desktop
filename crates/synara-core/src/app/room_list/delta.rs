//! Product room-list deltas (no SDK `VectorDiff` / `Room` on the wire).
//!
//! Mirrors the semantic shape of `eyeball_im::VectorDiff` so host projections
//! can apply ordered ops deterministically. Values are Synara [`RoomSummary`]
//! DTOs only — never raw matrix-sdk types.

use serde::{Deserialize, Serialize};

use crate::dto::RoomSummary;

/// One ordered mutation against a room-list projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RoomListDeltaOp {
    /// Replace the entire ordered list (initial snapshot or hard reset).
    Reset { rooms: Vec<RoomSummary> },
    /// Append rooms at the end.
    Append { rooms: Vec<RoomSummary> },
    /// Clear all rooms (empty list).
    Clear,
    /// Push one room at the front.
    PushFront { room: RoomSummary },
    /// Push one room at the back.
    PushBack { room: RoomSummary },
    /// Insert at index (0..=len).
    Insert { index: usize, room: RoomSummary },
    /// Replace room at index.
    Set { index: usize, room: RoomSummary },
    /// Remove room at index.
    Remove { index: usize },
    /// Keep only the first `len` rooms.
    Truncate { len: usize },
    /// Move room from `from` to `to`.
    Move { from: usize, to: usize },
}

impl RoomListDeltaOp {
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
/// `sequence` is monotonic per generation. Gap ⇒ resync (full `Reset` snapshot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomListDeltaBatch {
    pub session_generation: u64,
    pub sequence: u64,
    pub ops: Vec<RoomListDeltaOp>,
}

/// Full snapshot envelope (topic-compatible with IPC room_list body).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomListSnapshot {
    pub session_generation: u64,
    pub sequence: u64,
    pub rooms: Vec<RoomSummary>,
}

impl RoomListSnapshot {
    pub fn empty(session_generation: u64) -> Self {
        Self {
            session_generation,
            sequence: 0,
            rooms: Vec::new(),
        }
    }

    pub fn into_reset_batch(self) -> RoomListDeltaBatch {
        RoomListDeltaBatch {
            session_generation: self.session_generation,
            sequence: self.sequence,
            ops: vec![RoomListDeltaOp::Reset { rooms: self.rooms }],
        }
    }
}
