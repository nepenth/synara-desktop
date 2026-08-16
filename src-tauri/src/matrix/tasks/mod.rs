//! P2.4 — Task supervision and cancellation (src-tauri adapter module).
//!
//! SNC-P1-4: the task registry and supervisor-generation bridge live in
//! `crates/synara-core/src/task`. This module re-exports those types so every
//! `crate::matrix::tasks::…` path keeps resolving.
//!
//! **Harness / unit tests only until cutover.** No production login/sync loop,
//! no Tauri Matrix commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.4-task-supervision.md`

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::task as task_inner;
pub use synara_core::task::{
    matrix_tasks_markers, TaskError, TaskId, TaskInfo, TaskKind, TaskOutcome, TaskRunState,
    TaskSupervisor, MATRIX_TASKS_MARKER,
};

pub use synara_core::task::{follow_supervisor_generation, mirror_generation};

#[cfg(test)]
mod tests;
