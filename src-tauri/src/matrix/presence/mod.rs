//! P4.7 — Presence stream index and V-PRESENCE.USER native owner.
//!
//! Pure projection of per-user presence state. Complements P6.3 typing.
//! The product owner consumes the managed SDK client's global presence stream.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.7-presence.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
pub mod live;

pub use error::PresenceError;
pub use index::{
    PresenceIndex, PresenceSnapshot, PresenceState, MAX_PRESENCE_USERS, MAX_STATUS_MSG_CHARS,
};
pub use live::{
    NativePresenceOwner, NativePresenceSnapshot, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription, NativePresenceUpdate, NativePresenceUpdateOutcome,
    PRESENCE_UPDATED_EVENT,
};

/// Static marker for link / schema smoke.
pub const MATRIX_PRESENCE_MARKER: &str = "matrix-presence-p4.7";

/// Touch presence paths so they remain linked in non-test builds.
pub fn matrix_presence_markers() -> &'static str {
    let idx = PresenceIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(MAX_PRESENCE_USERS, 512);
    debug_assert_eq!(PresenceState::Online.as_str(), "online");
    debug_assert_eq!(MATRIX_PRESENCE_MARKER, "matrix-presence-p4.7");
    MATRIX_PRESENCE_MARKER
}

#[cfg(test)]
mod tests;
