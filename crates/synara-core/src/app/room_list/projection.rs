//! Deterministic room-list projection (snapshot + ordered deltas).
//!
//! Pure state machine for host/UI reconstruction tests. No network, no SDK
//! objects. Sequence gaps and generation mismatches force resync.

use crate::dto::RoomSummary;
use crate::transport::MatrixIpcErrorCategory;

use super::delta::{RoomListDeltaBatch, RoomListDeltaOp, RoomListSnapshot};
use super::error::RoomListError;

/// Live ordered room-list projection for one session generation.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomListProjection {
    session_generation: u64,
    /// Last applied sequence (0 = only empty/bootstrap).
    last_sequence: u64,
    rooms: Vec<RoomSummary>,
    /// True after a gap/invalid op until a Reset is applied.
    resync_required: bool,
}

impl RoomListProjection {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            last_sequence: 0,
            rooms: Vec::new(),
            resync_required: false,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn rooms(&self) -> &[RoomSummary] {
        &self.rooms
    }

    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    pub fn resync_required(&self) -> bool {
        self.resync_required
    }

    /// Export current state as a privacy-safe snapshot envelope.
    pub fn snapshot(&self) -> RoomListSnapshot {
        RoomListSnapshot {
            session_generation: self.session_generation,
            sequence: self.last_sequence,
            rooms: self.rooms.clone(),
        }
    }

    /// Install a full snapshot (always allowed; clears resync flag).
    pub fn apply_snapshot(&mut self, snap: RoomListSnapshot) -> Result<(), RoomListError> {
        if snap.session_generation != self.session_generation {
            return Err(RoomListError::StaleGeneration {
                diagnostic_id: "p4.2-snapshot-stale-generation",
                expected: self.session_generation,
                observed: snap.session_generation,
            });
        }
        self.rooms = snap.rooms;
        self.last_sequence = snap.sequence;
        self.resync_required = false;
        Ok(())
    }

    /// Apply an ordered delta batch.
    ///
    /// Rules:
    /// - generation must match
    /// - if `resync_required`, only a batch whose first op is `Reset` is accepted
    /// - `sequence` must be `last_sequence + 1` (except Reset may jump forward)
    pub fn apply_batch(&mut self, batch: RoomListDeltaBatch) -> Result<(), RoomListError> {
        if batch.session_generation != self.session_generation {
            self.resync_required = true;
            return Err(RoomListError::StaleGeneration {
                diagnostic_id: "p4.2-batch-stale-generation",
                expected: self.session_generation,
                observed: batch.session_generation,
            });
        }

        if batch.ops.is_empty() {
            return Err(RoomListError::Invalid {
                diagnostic_id: "p4.2-empty-batch",
            });
        }

        let first_is_reset = matches!(batch.ops.first(), Some(RoomListDeltaOp::Reset { .. }));

        if self.resync_required && !first_is_reset {
            return Err(RoomListError::ResyncRequired {
                diagnostic_id: "p4.2-resync-pending",
                // Stream resync reasons are modeled separately; IPC category stays stable.
                category: MatrixIpcErrorCategory::SdkInvariant,
            });
        }

        if first_is_reset {
            // Reset may re-baseline sequence.
        } else if batch.sequence != self.last_sequence.saturating_add(1) {
            self.resync_required = true;
            return Err(RoomListError::ResyncRequired {
                diagnostic_id: "p4.2-sequence-gap",
                category: MatrixIpcErrorCategory::SdkInvariant,
            });
        }

        for op in &batch.ops {
            if let Err(e) = apply_op(&mut self.rooms, op) {
                self.resync_required = true;
                return Err(e);
            }
        }

        self.last_sequence = batch.sequence;
        self.resync_required = false;
        Ok(())
    }
}

fn apply_op(rooms: &mut Vec<RoomSummary>, op: &RoomListDeltaOp) -> Result<(), RoomListError> {
    match op {
        RoomListDeltaOp::Reset { rooms: next } => {
            *rooms = next.clone();
            Ok(())
        }
        RoomListDeltaOp::Clear => {
            rooms.clear();
            Ok(())
        }
        RoomListDeltaOp::Append { rooms: more } => {
            rooms.extend(more.iter().cloned());
            Ok(())
        }
        RoomListDeltaOp::PushFront { room } => {
            rooms.insert(0, room.clone());
            Ok(())
        }
        RoomListDeltaOp::PushBack { room } => {
            rooms.push(room.clone());
            Ok(())
        }
        RoomListDeltaOp::Insert { index, room } => {
            if *index > rooms.len() {
                return Err(RoomListError::InvalidDelta {
                    diagnostic_id: "p4.2-insert-oob",
                });
            }
            rooms.insert(*index, room.clone());
            Ok(())
        }
        RoomListDeltaOp::Set { index, room } => {
            if *index >= rooms.len() {
                return Err(RoomListError::InvalidDelta {
                    diagnostic_id: "p4.2-set-oob",
                });
            }
            rooms[*index] = room.clone();
            Ok(())
        }
        RoomListDeltaOp::Remove { index } => {
            if *index >= rooms.len() {
                return Err(RoomListError::InvalidDelta {
                    diagnostic_id: "p4.2-remove-oob",
                });
            }
            rooms.remove(*index);
            Ok(())
        }
        RoomListDeltaOp::Truncate { len } => {
            if *len > rooms.len() {
                return Err(RoomListError::InvalidDelta {
                    diagnostic_id: "p4.2-truncate-oob",
                });
            }
            rooms.truncate(*len);
            Ok(())
        }
        RoomListDeltaOp::Move { from, to } => {
            if *from >= rooms.len() || *to >= rooms.len() {
                return Err(RoomListError::InvalidDelta {
                    diagnostic_id: "p4.2-move-oob",
                });
            }
            let item = rooms.remove(*from);
            rooms.insert(*to, item);
            Ok(())
        }
    }
}

/// Re-apply a snapshot then a sequence of batches; used by property-style tests.
pub fn reconstruct(
    session_generation: u64,
    snapshot: RoomListSnapshot,
    batches: &[RoomListDeltaBatch],
) -> Result<RoomListProjection, RoomListError> {
    let mut proj = RoomListProjection::new(session_generation);
    proj.apply_snapshot(snapshot)?;
    for batch in batches {
        proj.apply_batch(batch.clone())?;
    }
    Ok(proj)
}
