//! P8.7 — UTD retry / encrypted-history recovery foundation (harness).
//!
//! Room-level recovery coordinator. **No keys / event bodies.** Complements
//! P5.10 per-event UTD index. No SDK crypto, no dual-backend.
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

#[cfg(test)]
mod tests;
