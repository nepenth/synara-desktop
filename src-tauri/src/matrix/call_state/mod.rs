//! P10.4 — MatrixRTC membership / call-state projection foundation.
//!
//! Pure, generation-stamped call summaries for product UI. No live MatrixRTC,
//! experimental widgets, production commands, or dual-backend behavior.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p10.4-call-state.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::CallStateError;
pub use index::{
    CallMember, CallMembership, CallPhase, CallSessionSummary, CallStateIndex, MAX_CALL_MEMBERS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_CALL_STATE_MARKER: &str = "matrix-call-state-p10.4";

/// Touch call-state paths so they remain linked in non-test builds.
pub fn matrix_call_state_markers() -> &'static str {
    let index = CallStateIndex::new(0);
    debug_assert!(index.is_empty());
    debug_assert_eq!(index.len(), 0);
    debug_assert_eq!(MATRIX_CALL_STATE_MARKER, "matrix-call-state-p10.4");
    MATRIX_CALL_STATE_MARKER
}

#[cfg(test)]
mod tests;
