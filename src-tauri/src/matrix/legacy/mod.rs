//! P3.7 — Legacy-session detection and transition coordinator (harness).
//!
//! Clean-break reauth path. **Never starts matrix-js-sdk.** No token continuity,
//! no dual-backend. Failed transition preserves inert legacy data.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p3.7-legacy-transition.md`
//! Policy: `docs/matrix-rust-sdk/migration-ux-decision.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod transition;

pub use error::LegacyError;
pub use transition::{
    LegacyDetectionSignal, LegacySignalKind, LegacyTransition, TransitionCopyKey, TransitionPhase,
    MAX_DETECTION_SIGNALS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_LEGACY_MARKER: &str = "matrix-legacy-p3.7";

/// Touch legacy transition paths so they remain linked in non-test builds.
pub fn matrix_legacy_markers() -> &'static str {
    let t = LegacyTransition::new(0);
    debug_assert!(t.forbids_js_client_start());
    debug_assert!(t.forbids_token_continuity());
    debug_assert!(t.forbids_dual_backend());
    debug_assert_eq!(t.phase(), TransitionPhase::Idle);
    debug_assert_eq!(MATRIX_LEGACY_MARKER, "matrix-legacy-p3.7");
    MATRIX_LEGACY_MARKER
}

#[cfg(test)]
mod tests;
