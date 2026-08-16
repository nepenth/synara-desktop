//! Task registry + supervisor: spawn, cancel, join, generation isolation.
//!
//! **Harness / unit tests until cutover.** Does not start production login or
//! sync loops. Callers must stamp every task with the live session generation
//! (desktop: `crate::matrix::supervisor::MatrixSupervisor::session_generation`).

use std::collections::HashMap;
use std::future::Future;

use tokio::task::JoinHandle;

use super::error::TaskError;
use super::kind::TaskKind;

/// Opaque supervised-task identifier (monotonic within one [`TaskSupervisor`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    pub fn get(self) -> u64 {
        self.0
    }

    /// Construct from a raw id (tests / diagnostics only).
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// Terminal or running status for a registered task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskRunState {
    /// Future is still live (or placeholder not yet cancelled).
    Running,
    /// Future finished successfully.
    Completed,
    /// Cancel requested and/or abort observed.
    Cancelled,
    /// Join observed a non-cancel failure (panic / join error).
    Failed,
}

/// Result returned from [`TaskSupervisor::join`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskOutcome {
    Completed,
    Cancelled,
    Failed,
}

impl From<TaskOutcome> for TaskRunState {
    fn from(value: TaskOutcome) -> Self {
        match value {
            TaskOutcome::Completed => Self::Completed,
            TaskOutcome::Cancelled => Self::Cancelled,
            TaskOutcome::Failed => Self::Failed,
        }
    }
}

/// Read-only view of a registered task (no handles, no secrets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskInfo {
    pub id: TaskId,
    pub kind: TaskKind,
    pub generation: u64,
    pub state: TaskRunState,
}

struct TaskRecord {
    kind: TaskKind,
    generation: u64,
    state: TaskRunState,
    /// Present while the async task has not been joined.
    handle: Option<JoinHandle<TaskOutcome>>,
    /// Placeholder entries (pure registry) have no runtime handle.
    placeholder: bool,
}

/// Tracks supervised async work with session-generation isolation.
///
/// Invariants:
/// - Spawn/register only accept the **live** generation.
/// - [`Self::accept_result`] rejects stale generations.
/// - Generation bump + [`Self::retire_stale`] cancel and join old work so
///   superseded publishers cannot complete into a new epoch.
pub struct TaskSupervisor {
    next_id: u64,
    live_generation: u64,
    tasks: HashMap<TaskId, TaskRecord>,
    spawned_total: u64,
    joined_total: u64,
    cancelled_requests: u64,
}

impl Default for TaskSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskSupervisor {
    /// Empty registry, live generation 0 (aligned with a fresh MatrixSupervisor).
    pub fn new() -> Self {
        Self {
            next_id: 1,
            live_generation: 0,
            tasks: HashMap::new(),
            spawned_total: 0,
            joined_total: 0,
            cancelled_requests: 0,
        }
    }

    pub fn live_generation(&self) -> u64 {
        self.live_generation
    }

    /// Set the live generation without cancelling tasks.
    ///
    /// Prefer [`Self::bump_generation`] or the src-tauri bridge
    /// `follow_supervisor_generation` so stale work is retired.
    pub fn set_live_generation(&mut self, generation: u64) {
        self.live_generation = generation;
    }

    /// Increment live generation by one; returns the new value.
    ///
    /// Does **not** cancel tasks — call [`Self::retire_stale`] (async) after.
    pub fn bump_generation(&mut self) -> u64 {
        self.live_generation = self.live_generation.saturating_add(1);
        self.live_generation
    }

    pub fn spawned_total(&self) -> u64 {
        self.spawned_total
    }

    pub fn joined_total(&self) -> u64 {
        self.joined_total
    }

    pub fn cancelled_requests(&self) -> u64 {
        self.cancelled_requests
    }

    /// Number of tasks still present in the registry (running or terminal but not removed).
    pub fn registered_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn running_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|r| r.state == TaskRunState::Running)
            .count()
    }

    pub fn count_for_generation(&self, generation: u64) -> usize {
        self.tasks
            .values()
            .filter(|r| r.generation == generation)
            .count()
    }

    pub fn count_for_kind(&self, kind: TaskKind) -> usize {
        self.tasks.values().filter(|r| r.kind == kind).count()
    }

    pub fn get(&self, id: TaskId) -> Option<TaskInfo> {
        self.tasks.get(&id).map(|r| TaskInfo {
            id,
            kind: r.kind,
            generation: r.generation,
            state: r.state,
        })
    }

    pub fn list(&self) -> Vec<TaskInfo> {
        let mut out: Vec<TaskInfo> = self
            .tasks
            .iter()
            .map(|(id, r)| TaskInfo {
                id: *id,
                kind: r.kind,
                generation: r.generation,
                state: r.state,
            })
            .collect();
        out.sort_by_key(|t| t.id.get());
        out
    }

    /// True when `observed` equals the live session generation.
    pub fn is_live_generation(&self, observed: u64) -> bool {
        self.live_generation == observed
    }

    /// Refuse results stamped with a superseded session generation.
    pub fn accept_result(&self, generation: u64) -> Result<(), TaskError> {
        if generation != self.live_generation {
            return Err(TaskError::StaleGeneration {
                observed: generation,
                live: self.live_generation,
            });
        }
        Ok(())
    }

    /// Accept a result only if the task exists and its generation is still live.
    pub fn accept_task_result(&self, id: TaskId) -> Result<(), TaskError> {
        let rec = self.tasks.get(&id).ok_or(TaskError::UnknownTask { id })?;
        if rec.generation != self.live_generation {
            return Err(TaskError::StaleGeneration {
                observed: rec.generation,
                live: self.live_generation,
            });
        }
        Ok(())
    }

    /// Pure registry entry without a runtime future (generation bookkeeping tests).
    pub fn register(&mut self, kind: TaskKind, generation: u64) -> Result<TaskId, TaskError> {
        self.ensure_live_spawn(generation)?;
        let id = self.alloc_id();
        self.tasks.insert(
            id,
            TaskRecord {
                kind,
                generation,
                state: TaskRunState::Running,
                handle: None,
                placeholder: true,
            },
        );
        self.spawned_total = self.spawned_total.saturating_add(1);
        Ok(id)
    }

    /// Spawn supervised async work stamped with `generation`.
    ///
    /// Requires an ambient Tokio runtime. Refuses stale generations.
    pub fn spawn<F>(&mut self, kind: TaskKind, generation: u64, fut: F) -> Result<TaskId, TaskError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.ensure_live_spawn(generation)?;
        let id = self.alloc_id();
        let handle = tokio::spawn(async move {
            fut.await;
            TaskOutcome::Completed
        });
        self.tasks.insert(
            id,
            TaskRecord {
                kind,
                generation,
                state: TaskRunState::Running,
                handle: Some(handle),
                placeholder: false,
            },
        );
        self.spawned_total = self.spawned_total.saturating_add(1);
        Ok(id)
    }

    /// Request cancellation. Idempotent for already-cancelled or terminal tasks.
    pub fn cancel(&mut self, id: TaskId) -> Result<(), TaskError> {
        let rec = self
            .tasks
            .get_mut(&id)
            .ok_or(TaskError::UnknownTask { id })?;

        match rec.state {
            TaskRunState::Running => {
                if let Some(handle) = rec.handle.as_ref() {
                    handle.abort();
                }
                rec.state = TaskRunState::Cancelled;
                self.cancelled_requests = self.cancelled_requests.saturating_add(1);
                Ok(())
            }
            TaskRunState::Cancelled | TaskRunState::Completed | TaskRunState::Failed => {
                // Double-cancel / cancel-after-complete are no-ops (idempotent).
                Ok(())
            }
        }
    }

    /// Cancel every task with the given generation. Returns how many were running.
    pub fn cancel_generation(&mut self, generation: u64) -> usize {
        let ids: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|(_, r)| r.generation == generation && r.state == TaskRunState::Running)
            .map(|(id, _)| *id)
            .collect();
        let n = ids.len();
        for id in ids {
            let _ = self.cancel(id);
        }
        n
    }

    /// Cancel every registered running task.
    pub fn cancel_all(&mut self) -> usize {
        let ids: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|(_, r)| r.state == TaskRunState::Running)
            .map(|(id, _)| *id)
            .collect();
        let n = ids.len();
        for id in ids {
            let _ = self.cancel(id);
        }
        n
    }

    /// Await a single task and remove it from the registry.
    pub async fn join(&mut self, id: TaskId) -> Result<TaskOutcome, TaskError> {
        let mut rec = self
            .tasks
            .remove(&id)
            .ok_or(TaskError::UnknownTask { id })?;

        let outcome = if rec.placeholder {
            match rec.state {
                TaskRunState::Running => TaskOutcome::Completed,
                TaskRunState::Cancelled => TaskOutcome::Cancelled,
                TaskRunState::Completed => TaskOutcome::Completed,
                TaskRunState::Failed => TaskOutcome::Failed,
            }
        } else if let Some(handle) = rec.handle.take() {
            match handle.await {
                Ok(outcome) => outcome,
                Err(join_err) if join_err.is_cancelled() => TaskOutcome::Cancelled,
                Err(_) => TaskOutcome::Failed,
            }
        } else {
            // Handle already taken — treat as already joined.
            self.tasks.insert(id, rec);
            return Err(TaskError::AlreadyJoined { id });
        };

        rec.state = TaskRunState::from(outcome);
        self.joined_total = self.joined_total.saturating_add(1);
        Ok(outcome)
    }

    /// Cancel then join one task (idempotent cancel).
    pub async fn cancel_and_join(&mut self, id: TaskId) -> Result<TaskOutcome, TaskError> {
        // Unknown id fails at cancel; already-terminal cancel is ok.
        self.cancel(id)?;
        self.join(id).await
    }

    /// Cancel + join all tasks for `generation`; remove them from the registry.
    ///
    /// Returns the number of tasks retired.
    pub async fn retire_generation(&mut self, generation: u64) -> usize {
        let ids: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|(_, r)| r.generation == generation)
            .map(|(id, _)| *id)
            .collect();
        let n = ids.len();
        for id in ids {
            let _ = self.cancel_and_join(id).await;
        }
        n
    }

    /// Cancel + join every task whose generation is not the live generation.
    pub async fn retire_stale(&mut self) -> usize {
        let live = self.live_generation;
        let ids: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|(_, r)| r.generation != live)
            .map(|(id, _)| *id)
            .collect();
        let n = ids.len();
        for id in ids {
            let _ = self.cancel_and_join(id).await;
        }
        n
    }

    /// Cancel + join every task (full shutdown for open/close cycles).
    pub async fn shutdown_all(&mut self) -> usize {
        let ids: Vec<TaskId> = self.tasks.keys().copied().collect();
        let n = ids.len();
        for id in ids {
            let _ = self.cancel_and_join(id).await;
        }
        debug_assert_eq!(self.tasks.len(), 0);
        n
    }

    fn ensure_live_spawn(&self, generation: u64) -> Result<(), TaskError> {
        if generation != self.live_generation {
            return Err(TaskError::SpawnStaleGeneration {
                observed: generation,
                live: self.live_generation,
            });
        }
        Ok(())
    }

    fn alloc_id(&mut self) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}
