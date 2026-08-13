//! P6.9 — Room membership / lifecycle ops queue foundation (harness).
//!
//! Tracks create, join, leave, invite, kick, ban, unban, forget intents.
//! No SDK network, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.9-room-ops.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod ipc;
mod queue;

pub use error::RoomOpsError;
pub use ipc::{
    MatrixRoomCreateContent, MatrixRoomCreatePowerLevels, MatrixRoomCreatePreset,
    MatrixRoomCreateRequest, MatrixRoomCreateVisibility,
};
pub use queue::{
    LocalOpId, RoomOp, RoomOpKind, RoomOpState, RoomOpsQueue, MAX_CREATE_NAME_CHARS,
    MAX_REASON_CHARS, MAX_TRACKED_OPS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_ROOM_OPS_MARKER: &str = "matrix-room-ops-p6.9";

/// Touch room-ops paths so they remain linked in non-test builds.
pub fn matrix_room_ops_markers() -> &'static str {
    let q = RoomOpsQueue::new(0);
    debug_assert!(q.is_empty());
    debug_assert_eq!(RoomOpKind::Join.as_str(), "join");
    debug_assert_eq!(MATRIX_ROOM_OPS_MARKER, "matrix-room-ops-p6.9");
    MATRIX_ROOM_OPS_MARKER
}

#[cfg(test)]
mod tests;
