//! Native room-key import/export ownership.
//!
//! The transfer flow stores counts and privacy-safe labels only. Live SDK and
//! host-file ownership lives in the desktop shell; room keys, passphrases, file
//! bytes, and import paths never appear in command results.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.6-room-key-export.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod flow;
mod native;

pub use error::RoomKeyError;
pub use flow::{
    RoomKeyTransferFlow, RoomKeyTransferKind, RoomKeyTransferOutcome, RoomKeyTransferPhase,
};
pub use native::{
    project_room_key_status, NativeRoomKeyFileSelection, NativeRoomKeyTransferKind,
    NativeRoomKeyTransferPhase, NativeRoomKeyTransferResult, NativeRoomKeyTransferStatus,
    EXPORT_FILE_NAME,
};

/// Static marker for link / schema smoke.
pub const MATRIX_ROOM_KEYS_MARKER: &str = "matrix-room-keys-p8.6";

/// Touch room-key transfer paths so they remain linked in non-test builds.
pub fn matrix_room_keys_markers() -> &'static str {
    let flow = RoomKeyTransferFlow::new(0);
    debug_assert!(!flow.is_active());
    debug_assert_eq!(flow.phase(), RoomKeyTransferPhase::Idle);
    debug_assert_eq!(RoomKeyTransferKind::Export.as_str(), "export");
    debug_assert_eq!(MATRIX_ROOM_KEYS_MARKER, "matrix-room-keys-p8.6");
    MATRIX_ROOM_KEYS_MARKER
}

#[cfg(test)]
mod tests;
