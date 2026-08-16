//! Task supervision errors (privacy-safe; no tokens / secrets).

use super::TaskId;

/// Failure from the task supervisor registry or generation checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// Task id is not present in the registry.
    UnknownTask { id: TaskId },
    /// Observed session generation does not match the live generation.
    StaleGeneration { observed: u64, live: u64 },
    /// Spawn/register refused because the supplied generation is not live.
    SpawnStaleGeneration { observed: u64, live: u64 },
    /// Join was requested but the task handle was already reaped.
    AlreadyJoined { id: TaskId },
}

impl TaskError {
    pub fn is_stale_generation(&self) -> bool {
        matches!(
            self,
            Self::StaleGeneration { .. } | Self::SpawnStaleGeneration { .. }
        )
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTask { id } => write!(f, "unknown task id {}", id.get()),
            Self::StaleGeneration { observed, live } => {
                write!(f, "stale task generation: observed={observed} live={live}")
            }
            Self::SpawnStaleGeneration { observed, live } => write!(
                f,
                "refuse spawn for stale generation: observed={observed} live={live}"
            ),
            Self::AlreadyJoined { id } => {
                write!(f, "task {} already joined", id.get())
            }
        }
    }
}

impl std::error::Error for TaskError {}
