//! P4.7 — Presence stream index and V-PRESENCE.USER native owner.
//!
//! Pure projection of per-user presence state. Complements P6.3 typing.
//! The product owner consumes the managed SDK client's global presence stream.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.7-presence.md`

pub use synara_core::app::presence::*;

pub mod live;
pub use live::{
    NativePresenceOwner, NativePresenceSnapshot, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription, NativePresenceUpdate, NativePresenceUpdateOutcome,
    PRESENCE_UPDATED_EVENT,
};
