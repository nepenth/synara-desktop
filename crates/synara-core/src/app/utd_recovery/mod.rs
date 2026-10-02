//! P8.7 — UTD retry / encrypted-history recovery foundation (shared native core).
//!
//! Room-level recovery coordinator. **No keys / event bodies.** Complements
//! P5.10 per-event UTD index. No SDK crypto, no dual-backend.
//!
//! SNC-P1-5c: moved from src-tauri `matrix/utd_recovery` into the shared core
//! because `app::timeline::live` (NativeTimelineRegistry) has a hard type
//! dependency on [`UtdRecoveryCoordinator`]. src-tauri's
//! `matrix/utd_recovery/mod.rs` is now a re-export adapter so every
//! `crate::matrix::utd_recovery::*` path keeps resolving identically.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.7-utd-recovery.md`

mod error;
mod flow;

pub use error::UtdRecoveryError;
pub use flow::{
    UtdRecoveryCoordinator, UtdRecoveryKind, UtdRecoveryPhase, UtdRecoverySession,
    MAX_EVENT_IDS_PER_BATCH, MAX_ROOM_SESSIONS,
};
