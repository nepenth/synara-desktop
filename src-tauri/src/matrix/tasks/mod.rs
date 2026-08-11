//! P2.4 — Task supervision and cancellation (src-tauri adapter module).
//!
//! SNC-P1-4: the pure task registry lives in `crates/synara-core/src/task`
//! (`task_inner`). This module keeps every `crate::matrix::tasks::…` path
//! resolving with **identical behavior** by re-exporting the core types, and
//! keeps the desktop-only `bridge` adapter that composes the core
//! [`TaskSupervisor`] with the (still desktop-local, P1.5 "rest" chunk)
//! [`crate::matrix::supervisor::MatrixSupervisor`].
//!
//! **Harness / unit tests only until cutover.** No production login/sync loop,
//! no Tauri Matrix commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.4-task-supervision.md`

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::task as task_inner;
pub use synara_core::task::{
    matrix_tasks_markers, TaskError, TaskId, TaskInfo, TaskKind, TaskOutcome,
    TaskRunState, TaskSupervisor, MATRIX_TASKS_MARKER,
};

mod bridge;
pub use bridge::{follow_supervisor_generation, mirror_generation};

#[cfg(test)]
mod tests;
