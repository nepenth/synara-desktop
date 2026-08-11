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

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod flow;

pub use error::UtdRecoveryError;
pub use flow::{
    UtdRecoveryCoordinator, UtdRecoveryKind, UtdRecoveryPhase, UtdRecoverySession,
    MAX_EVENT_IDS_PER_BATCH, MAX_ROOM_SESSIONS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_UTD_RECOVERY_MARKER: &str = "matrix-utd-recovery-p8.7";

/// Touch UTD recovery paths so they remain linked in non-test builds.
pub fn matrix_utd_recovery_markers() -> &'static str {
    let c = UtdRecoveryCoordinator::new(0);
    debug_assert!(c.is_empty());
    debug_assert_eq!(UtdRecoveryKind::RetryDecrypt.as_str(), "retry_decrypt");
    debug_assert_eq!(MATRIX_UTD_RECOVERY_MARKER, "matrix-utd-recovery-p8.7");
    MATRIX_UTD_RECOVERY_MARKER
}
