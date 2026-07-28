//! P5.7 — Poll and state/membership event projection foundation (harness).
//!
//! Pure indexes for MSC3381-style polls plus room state and membership timeline
//! items. No SDK send, no production Tauri commands, no dual-backend, no secrets
//! in errors or summaries.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.7-polls-state.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::PollError;
pub use index::{
    MembershipEventIndex, PollAnswer, PollIndex, PollPhase, PollSummary, StateEventIndex,
    MAX_ANSWERS_PER_POLL, MAX_MEMBERSHIP_EVENTS, MAX_POLLS_PER_ROOM, MAX_STATE_KEYS_PER_ROOM,
};

/// Static marker for link / schema smoke.
pub const MATRIX_POLLS_MARKER: &str = "matrix-polls-state-p5.7";

/// Touch poll/state paths so they remain linked in non-test builds.
pub fn matrix_polls_markers() -> &'static str {
    let polls = PollIndex::new(0);
    debug_assert!(polls.is_empty());
    let state = StateEventIndex::new(0);
    debug_assert!(state.is_empty());
    let mem = MembershipEventIndex::new(0);
    debug_assert!(mem.is_empty());
    debug_assert_eq!(PollPhase::Open.as_str(), "open");
    debug_assert_eq!(MATRIX_POLLS_MARKER, "matrix-polls-state-p5.7");
    MATRIX_POLLS_MARKER
}

#[cfg(test)]
mod tests;
