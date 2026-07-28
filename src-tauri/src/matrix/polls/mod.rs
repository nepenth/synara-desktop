//! P5.7 — Poll and room state/membership projection foundation (harness).
//!
//! Pure, generation-stamped indexes of poll summary rows and simple room state
//! summary rows. No SDK timeline wiring, send, production Tauri commands, or
//! dual-backend behavior.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.7-poll-state-projection.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod model;

pub use error::ProjectionError;
pub use index::{PollIndex, StateProjectionIndex};
pub use model::{PollProjection, StateProjectionKind, StateProjectionRow};

/// Static marker for link / schema smoke.
pub const MATRIX_POLLS_MARKER: &str = "matrix-polls-p5.7";

/// Touch poll/state projection paths so they remain linked in non-test builds.
pub fn matrix_polls_markers() -> &'static str {
    let polls = PollIndex::new(0);
    let state = StateProjectionIndex::new(0);
    debug_assert!(polls.is_empty());
    debug_assert!(state.is_empty());
    debug_assert_eq!(MATRIX_POLLS_MARKER, "matrix-polls-p5.7");
    MATRIX_POLLS_MARKER
}

#[cfg(test)]
mod tests;
