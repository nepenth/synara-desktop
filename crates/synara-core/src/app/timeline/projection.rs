//! Deterministic timeline projection (snapshot + ordered deltas).
//!
//! Pure state machine for host/UI reconstruction tests. No network, no SDK
//! timeline objects. Sequence gaps and generation mismatches force resync.
//! SDK `Timeline` attach/diff mapping remains a later slice.

use crate::dto::TimelineItem;
use crate::transport::MatrixIpcErrorCategory;

use super::delta::{TimelineDeltaBatch, TimelineDeltaOp, TimelineSnapshot};
use super::error::TimelineError;

/// Live ordered timeline projection for one session generation (one stream).
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineProjection {
    session_generation: u64,
    /// Last applied sequence (0 = only empty/bootstrap).
    last_sequence: u64,
    items: Vec<TimelineItem>,
    /// True after a gap/invalid op until a Reset is applied.
    resync_required: bool,
}

impl TimelineProjection {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            last_sequence: 0,
            items: Vec::new(),
            resync_required: false,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn items(&self) -> &[TimelineItem] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn resync_required(&self) -> bool {
        self.resync_required
    }

    /// Export current state as a privacy-safe snapshot envelope.
    pub fn snapshot(&self) -> TimelineSnapshot {
        TimelineSnapshot {
            session_generation: self.session_generation,
            sequence: self.last_sequence,
            items: self.items.clone(),
        }
    }

    /// Install a full snapshot (always allowed; clears resync flag).
    pub fn apply_snapshot(&mut self, snap: TimelineSnapshot) -> Result<(), TimelineError> {
        if snap.session_generation != self.session_generation {
            return Err(TimelineError::StaleGeneration {
                diagnostic_id: "p5.2-snapshot-stale-generation",
                expected: self.session_generation,
                observed: snap.session_generation,
            });
        }
        self.items = snap.items;
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
    pub fn apply_batch(&mut self, batch: TimelineDeltaBatch) -> Result<(), TimelineError> {
        if batch.session_generation != self.session_generation {
            self.resync_required = true;
            return Err(TimelineError::StaleGeneration {
                diagnostic_id: "p5.2-batch-stale-generation",
                expected: self.session_generation,
                observed: batch.session_generation,
            });
        }

        if batch.ops.is_empty() {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.2-empty-batch",
            });
        }

        let first_is_reset = matches!(batch.ops.first(), Some(TimelineDeltaOp::Reset { .. }));

        if self.resync_required && !first_is_reset {
            return Err(TimelineError::ResyncRequired {
                diagnostic_id: "p5.2-resync-pending",
                category: MatrixIpcErrorCategory::SdkInvariant,
            });
        }

        if first_is_reset {
            // Reset may re-baseline sequence.
        } else if batch.sequence != self.last_sequence.saturating_add(1) {
            self.resync_required = true;
            return Err(TimelineError::ResyncRequired {
                diagnostic_id: "p5.2-sequence-gap",
                category: MatrixIpcErrorCategory::SdkInvariant,
            });
        }

        for op in &batch.ops {
            if let Err(e) = apply_op(&mut self.items, op) {
                self.resync_required = true;
                return Err(e);
            }
        }

        self.last_sequence = batch.sequence;
        self.resync_required = false;
        Ok(())
    }
}

fn apply_op(items: &mut Vec<TimelineItem>, op: &TimelineDeltaOp) -> Result<(), TimelineError> {
    match op {
        TimelineDeltaOp::Reset { items: next } => {
            *items = next.clone();
            Ok(())
        }
        TimelineDeltaOp::Clear => {
            items.clear();
            Ok(())
        }
        TimelineDeltaOp::Append { items: more } => {
            items.extend(more.iter().cloned());
            Ok(())
        }
        TimelineDeltaOp::PushFront { item } => {
            items.insert(0, item.clone());
            Ok(())
        }
        TimelineDeltaOp::PushBack { item } => {
            items.push(item.clone());
            Ok(())
        }
        TimelineDeltaOp::Insert { index, item } => {
            if *index > items.len() {
                return Err(TimelineError::InvalidDelta {
                    diagnostic_id: "p5.2-insert-oob",
                });
            }
            items.insert(*index, item.clone());
            Ok(())
        }
        TimelineDeltaOp::Set { index, item } => {
            if *index >= items.len() {
                return Err(TimelineError::InvalidDelta {
                    diagnostic_id: "p5.2-set-oob",
                });
            }
            items[*index] = item.clone();
            Ok(())
        }
        TimelineDeltaOp::Remove { index } => {
            if *index >= items.len() {
                return Err(TimelineError::InvalidDelta {
                    diagnostic_id: "p5.2-remove-oob",
                });
            }
            items.remove(*index);
            Ok(())
        }
        TimelineDeltaOp::Truncate { len } => {
            if *len > items.len() {
                return Err(TimelineError::InvalidDelta {
                    diagnostic_id: "p5.2-truncate-oob",
                });
            }
            items.truncate(*len);
            Ok(())
        }
        TimelineDeltaOp::Move { from, to } => {
            if *from >= items.len() || *to >= items.len() {
                return Err(TimelineError::InvalidDelta {
                    diagnostic_id: "p5.2-move-oob",
                });
            }
            let item = items.remove(*from);
            items.insert(*to, item);
            Ok(())
        }
    }
}

/// Re-apply a snapshot then a sequence of batches; used by property-style tests.
pub fn reconstruct(
    session_generation: u64,
    snapshot: TimelineSnapshot,
    batches: &[TimelineDeltaBatch],
) -> Result<TimelineProjection, TimelineError> {
    let mut proj = TimelineProjection::new(session_generation);
    proj.apply_snapshot(snapshot)?;
    for batch in batches {
        proj.apply_batch(batch.clone())?;
    }
    Ok(proj)
}
