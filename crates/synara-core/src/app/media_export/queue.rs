//! Metadata-only save, share, open, and drag export intent queue (P7.5).
//!
//! The queue owns no media bytes and performs no filesystem or platform I/O.

use std::collections::HashMap;

use crate::dto::RoomId;

use super::error::ExportError;

/// Opaque identifier allocated for one export intent.
pub type ExportJobId = String;

/// Platform action requested for a media handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Save,
    Share,
    Open,
    Drag,
}

/// Lifecycle of an export intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ExportState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// Metadata-only media export job.
///
/// `media_handle_id` is opaque. It is deliberately redacted from `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct ExportJob {
    pub id: ExportJobId,
    pub kind: ExportKind,
    pub media_handle_id: String,
    pub room_id: Option<RoomId>,
    pub state: ExportState,
}

impl std::fmt::Debug for ExportJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportJob")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("media_handle_id", &"<opaque>")
            .field("room_id_present", &self.room_id.is_some())
            .field("state", &self.state)
            .finish()
    }
}

/// Session-generation-scoped queue of export intents.
#[derive(Debug, Default)]
pub struct ExportQueue {
    session_generation: u64,
    order: Vec<ExportJobId>,
    jobs: HashMap<ExportJobId, ExportJob>,
    next_seq: u64,
}

impl ExportQueue {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            order: Vec::new(),
            jobs: HashMap::new(),
            next_seq: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn alloc_id(&mut self) -> ExportJobId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("export-{}", self.next_seq)
    }

    /// Enqueue an export intent without reading media or touching the filesystem.
    pub fn enqueue(
        &mut self,
        kind: ExportKind,
        media_handle_id: impl Into<String>,
        room_id: Option<RoomId>,
    ) -> Result<&ExportJob, ExportError> {
        let media_handle_id = media_handle_id.into().trim().to_owned();
        if media_handle_id.is_empty() {
            return Err(ExportError::Invalid {
                diagnostic_id: "p7.5-empty-media-handle",
            });
        }
        if media_handle_id.len() > 4096 || media_handle_id.chars().any(char::is_control) {
            return Err(ExportError::Invalid {
                diagnostic_id: "p7.5-invalid-media-handle",
            });
        }
        if let Some(room_id) = room_id.as_deref() {
            if room_id.is_empty()
                || !room_id.starts_with('!')
                || room_id.chars().any(char::is_control)
            {
                return Err(ExportError::Invalid {
                    diagnostic_id: "p7.5-invalid-room-id",
                });
            }
        }

        let id = self.alloc_id();
        let job = ExportJob {
            id: id.clone(),
            kind,
            media_handle_id,
            room_id,
            state: ExportState::Pending,
        };
        self.order.push(id.clone());
        self.jobs.insert(id.clone(), job);
        Ok(self.jobs.get(&id).expect("job was just inserted"))
    }

    pub fn get(&self, id: &str) -> Option<&ExportJob> {
        self.jobs.get(id)
    }

    pub fn list(&self) -> Vec<&ExportJob> {
        self.order
            .iter()
            .filter_map(|id| self.jobs.get(id))
            .collect()
    }

    fn get_mut_checked(&mut self, id: &str) -> Result<&mut ExportJob, ExportError> {
        self.jobs.get_mut(id).ok_or(ExportError::NotFound {
            diagnostic_id: "p7.5-export-not-found",
        })
    }

    /// Transition a pending intent to running.
    pub fn start(&mut self, id: &str) -> Result<&ExportJob, ExportError> {
        let job = self.get_mut_checked(id)?;
        if job.state != ExportState::Pending {
            return Err(ExportError::Invalid {
                diagnostic_id: "p7.5-start-not-pending",
            });
        }
        job.state = ExportState::Running;
        Ok(job)
    }

    /// Transition a running intent to succeeded.
    pub fn complete(&mut self, id: &str) -> Result<&ExportJob, ExportError> {
        let job = self.get_mut_checked(id)?;
        if job.state != ExportState::Running {
            return Err(ExportError::Invalid {
                diagnostic_id: "p7.5-complete-not-running",
            });
        }
        job.state = ExportState::Succeeded;
        Ok(job)
    }

    /// Transition a pending or running intent to failed.
    pub fn fail(&mut self, id: &str) -> Result<&ExportJob, ExportError> {
        let job = self.get_mut_checked(id)?;
        if !matches!(job.state, ExportState::Pending | ExportState::Running) {
            return Err(ExportError::Invalid {
                diagnostic_id: "p7.5-fail-terminal",
            });
        }
        job.state = ExportState::Failed;
        Ok(job)
    }

    /// Transition a pending or running intent to cancelled.
    pub fn cancel(&mut self, id: &str) -> Result<&ExportJob, ExportError> {
        let job = self.get_mut_checked(id)?;
        if !matches!(job.state, ExportState::Pending | ExportState::Running) {
            return Err(ExportError::Invalid {
                diagnostic_id: "p7.5-cancel-terminal",
            });
        }
        job.state = ExportState::Cancelled;
        Ok(job)
    }

    /// Remove succeeded, failed, and cancelled jobs.
    pub fn prune_terminal(&mut self) -> usize {
        let before = self.jobs.len();
        self.order.retain(|id| {
            self.jobs
                .get(id)
                .map(|job| !job.state.is_terminal())
                .unwrap_or(false)
        });
        self.jobs.retain(|_, job| !job.state.is_terminal());
        before.saturating_sub(self.jobs.len())
    }

    /// Cancel active jobs and advance to a new session generation.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        for job in self.jobs.values_mut() {
            if matches!(job.state, ExportState::Pending | ExportState::Running) {
                job.state = ExportState::Cancelled;
            }
        }
    }
}
