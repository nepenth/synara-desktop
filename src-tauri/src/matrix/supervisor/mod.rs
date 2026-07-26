//! P2.1 — Matrix supervisor actor (lifecycle foundation).
//!
//! Single-owner state machine for the future Matrix Rust client. This module
//! is the **only** construction path for a client handle slot; product code
//! must not invent a second owner or dual-backend selector.
//!
//! **Harness / unit tests only until cutover.** No Tauri command registration,
//! no production login/sync loop. Live `Client::builder` lives only under
//! `matrix/client_builder/` (P2.3 unauthenticated open); login/sync remain
//! forbidden under all `matrix/` non-test modules.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.1-matrix-supervisor-actor.md`
//!
//! Lifecycle states: empty, opening, authenticating, restoring, syncing, ready,
//! stopping, logged_out, failed, wiping (`SupervisorState`, wire-aligned with
//! `dto::SessionLifecycle`).
//!
//! Async task cancel/join on generation bump is owned by
//! [`crate::matrix::tasks`] (P2.4) via
//! `tasks::follow_supervisor_generation` — this actor stays pure/sync.

#![allow(dead_code)]
#![allow(unused_imports)]

mod actor;
mod error;
mod handle;
mod state;
mod transition;

pub use actor::{
    harness_login_ready, harness_restore_ready, FailureInfo, MatrixSupervisor, SupervisorSnapshot,
};
pub use error::{SupervisorError, TransitionError};
pub use handle::{
    ClientFactory, ClientHandle, FactoryError, NullClientFactory, TestClientFactory,
    TestClientHandle,
};
pub use state::{SupervisorEvent, SupervisorState};
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
