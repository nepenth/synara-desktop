//! P4.7 — Presence stream index and V-PRESENCE.USER native owner.
//!
//! Pure presence projection plus live `NativePresenceOwner`.
//! Shells supply the emit sink (desktop Tauri event / later iOS UniFFI).
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.7-presence.md`

mod error;
mod index;
mod live;
mod native;

pub use error::PresenceError;
pub use index::{
    PresenceIndex, PresenceSnapshot, PresenceState, MAX_PRESENCE_TIMESTAMP_MS, MAX_PRESENCE_USERS,
    MAX_STATUS_MSG_CHARS,
};
pub use live::{NativePresenceOwner, PresenceUpdateEmit};
pub use native::{
    subscription_id_generation, NativePresenceSnapshot, NativePresenceSnapshotResult,
    NativePresenceState, NativePresenceSubscription, NativePresenceUpdate,
    NativePresenceUpdateOutcome, NativePresenceWriteResult, PresenceSubscriptionRegistry,
    PRESENCE_UPDATED_EVENT,
};

#[cfg(test)]
mod tests;
