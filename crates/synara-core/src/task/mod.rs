//! P2.4 — Task supervision and cancellation.
//!
//! Tracks supervised async work (sync / listener / upload / search / generic)
//! with **session-generation isolation**: tasks are stamped at spawn, cancelled
//! and joined on generation bump, and stale-generation results are refused.
//!
//! Standalone registry. Compose with [`crate::app::supervisor::MatrixSupervisor`]
//! via [`follow_supervisor_generation`] after lifecycle generation bumps.
//!
//! **Harness / unit tests only until cutover.** No production login/sync loop,
//! no Tauri Matrix commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.4-task-supervision.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod bridge;
mod error;
mod kind;
mod registry;

pub use bridge::{follow_supervisor_generation, mirror_generation};
pub use error::TaskError;
pub use kind::TaskKind;
pub use registry::{TaskId, TaskInfo, TaskOutcome, TaskRunState, TaskSupervisor};

/// Static marker for link / schema smoke (no network, no Client).
pub const MATRIX_TASKS_MARKER: &str = "matrix-task-supervision-p2.4";

/// Touch task-supervision paths so the foundation remains linked in non-test builds.
pub fn matrix_tasks_markers() -> &'static str {
    let _kinds = TaskKind::ALL.len();
    let supervisor = TaskSupervisor::new();
    debug_assert_eq!(_kinds, 5);
    debug_assert_eq!(supervisor.live_generation(), 0);
    debug_assert_eq!(supervisor.registered_count(), 0);
    debug_assert_eq!(MATRIX_TASKS_MARKER, "matrix-task-supervision-p2.4");
    MATRIX_TASKS_MARKER
}

#[cfg(test)]
mod tests;
