//! P4.2 — Room-list snapshot and delta stream (harness foundation).
//!
//! Deterministic projection of ordered room summaries for the dogfood path:
//! - product DTOs only ([`RoomSummary`]) — no SDK Room/VectorDiff on the wire
//! - ordered delta ops with monotonic sequence + session generation
//! - gap / stale generation → resync (full snapshot reset)
//!
//! **Harness / unit tests only until cutover.** No production Tauri commands,
//! no live sliding-sync subscription loop, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.2-room-list.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod delta;
mod error;
mod projection;
mod summary;

pub use delta::{RoomListDeltaBatch, RoomListDeltaOp, RoomListSnapshot};
pub use error::RoomListError;
pub use projection::{reconstruct, RoomListProjection};
pub use summary::RoomSummaryBuilder;

/// Static marker for link / schema smoke.
pub const MATRIX_ROOM_LIST_MARKER: &str = "matrix-room-list-snapshot-delta-p4.2";

/// Touch room-list foundation paths so they remain linked in non-test builds.
pub fn matrix_room_list_markers() -> &'static str {
    let mut proj = RoomListProjection::new(1);
    let snap = RoomListSnapshot::empty(1);
    let _ = proj.apply_snapshot(snap);
    debug_assert!(proj.is_empty());
    debug_assert_eq!(proj.last_sequence(), 0);
    debug_assert_eq!(RoomListDeltaOp::Clear.op_name(), "clear");
    debug_assert_eq!(
        MATRIX_ROOM_LIST_MARKER,
        "matrix-room-list-snapshot-delta-p4.2"
    );
    MATRIX_ROOM_LIST_MARKER
}

#[cfg(test)]
mod tests;
