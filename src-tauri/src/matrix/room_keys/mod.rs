//! P8.6 — Room-key import/export foundation (harness).
//!
//! Pure transfer flow. **Never stores room keys, passphrases, or file bytes.**
//! No SDK crypto APIs, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.6-room-key-export.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod flow;

pub use error::RoomKeyError;
pub use flow::{
    RoomKeyTransferFlow, RoomKeyTransferKind, RoomKeyTransferOutcome, RoomKeyTransferPhase,
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
