//! P2.1 — Matrix supervisor actor (lifecycle foundation).
//!
//! Single-owner state machine for the future Matrix Rust client. This module
//! is the **only** construction path for a client handle slot; product code
//! must not invent a second owner or dual-backend selector.
//!
//! **Harness / unit tests only until cutover.** No Tauri command registration,
//! no production login/sync loop, no `matrix_sdk::Client` builder (P2.3).
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.1-matrix-supervisor-actor.md`
//!
//! Lifecycle states: empty, opening, authenticating, restoring, syncing, ready,
//! stopping, logged_out, failed, wiping (`SupervisorState`, wire-aligned with
//! `dto::SessionLifecycle`).

#![allow(dead_code)]
#![allow(unused_imports)]

mod actor;
mod error;
mod handle;
mod state;
mod transition;

pub use actor::{MatrixSupervisor, SupervisorSnapshot};
pub use error::{SupervisorError, TransitionError};
pub use handle::{ClientFactory, ClientHandle, NullClientFactory, TestClientHandle};
pub use state::{SupervisorState, SupervisorEvent};
pub use transition::{SupervisorCommand, TransitionEffect};

/// Static marker for link / schema smoke (no network, no Client).
pub const MATRIX_SUPERVISOR_MARKER: &str = "matrix-supervisor-actor-p2.1";

/// Touch supervisor paths so the foundation remains linked in non-test builds.
pub fn matrix_supervisor_markers() -> &'static str {
    let _states = SupervisorState::ALL.len();
    let _events = SupervisorEvent::ALL.len();
    let _cmds = SupervisorCommand::ALL.len();
    let actor = MatrixSupervisor::new();
    debug_assert_eq!(_states, 10);
    debug_assert!(_events > 0);
    debug_assert!(_cmds > 0);
    debug_assert_eq!(actor.state(), SupervisorState::Empty);
    debug_assert_eq!(actor.session_generation(), 0);
    debug_assert_eq!(MATRIX_SUPERVISOR_MARKER, "matrix-supervisor-actor-p2.1");
    MATRIX_SUPERVISOR_MARKER
}

#[cfg(test)]
mod tests;
