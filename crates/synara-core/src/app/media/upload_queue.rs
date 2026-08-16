//! Media upload job queue (P6.4 harness foundation).
//!
//! Tracks [`UploadJob`] metadata and lifecycle only — **never** file bytes.
//! No SDK `Media::upload`, no dual-backend, no tokens in errors.

use std::collections::HashMap;

use crate::dto::{RoomId, UploadId, UploadJob, UploadState};

use super::error::MediaError;

/// Soft cap on concurrent active (queued+uploading) jobs.
pub const MAX_ACTIVE_UPLOADS: usize = 16;

/// Session-generation-stamped upload queue (metadata only).
#[derive(Debug, Default)]
pub struct UploadQueue {
    session_generation: u64,
    order: Vec<UploadId>,
    jobs: HashMap<UploadId, UploadJob>,
    next_seq: u64,
    /// Privacy-safe failure diagnostics by upload id (not on wire DTO).
    failures: HashMap<UploadId, &'static str>,
}

impl UploadQueue {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            order: Vec::new(),
            jobs: HashMap::new(),
            next_seq: 0,
            failures: HashMap::new(),
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

    pub fn active_count(&self) -> usize {
        self.jobs
            .values()
            .filter(|j| matches!(j.state, UploadState::Queued | UploadState::Uploading))
            .count()
    }

    fn alloc_id(&mut self) -> UploadId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("upload-{}", self.next_seq)
    }

    /// Enqueue a metadata-only upload job (no bytes).
    pub fn enqueue(
        &mut self,
        file_name: impl Into<String>,
        room_id: Option<RoomId>,
        mime_type: Option<String>,
        size_bytes: Option<u64>,
    ) -> Result<&UploadJob, MediaError> {
        let file_name = file_name.into().trim().to_owned();
        if file_name.is_empty() {
            return Err(MediaError::Invalid {
                diagnostic_id: "p6.4-empty-file-name",
            });
        }
        if file_name.len() > 1024 {
            return Err(MediaError::Invalid {
                diagnostic_id: "p6.4-file-name-too-long",
            });
        }
        if let Some(ref room) = room_id {
            if room.is_empty() || !room.starts_with('!') {
                return Err(MediaError::Invalid {
                    diagnostic_id: "p6.4-invalid-room-id",
                });
            }
        }
        if let Some(sz) = size_bytes {
            // Soft product guard (100 MiB) — host may tighten further.
            if sz > 100 * 1024 * 1024 {
                return Err(MediaError::Invalid {
                    diagnostic_id: "p6.4-file-too-large",
                });
            }
        }
        if self.active_count() >= MAX_ACTIVE_UPLOADS {
            return Err(MediaError::Invalid {
                diagnostic_id: "p6.4-active-upload-cap",
            });
        }
        let upload_id = self.alloc_id();
        let job = UploadJob {
            upload_id: upload_id.clone(),
            room_id,
            file_name,
            mime_type,
            size_bytes,
            state: UploadState::Queued,
            progress01: None,
            media_handle_id: None,
        };
        self.order.push(upload_id.clone());
        self.jobs.insert(upload_id.clone(), job);
        Ok(self.jobs.get(&upload_id).expect("just inserted"))
    }

    pub fn get(&self, upload_id: &str) -> Option<&UploadJob> {
        self.jobs.get(upload_id)
    }

    pub fn failure_diagnostic(&self, upload_id: &str) -> Option<&'static str> {
        self.failures.get(upload_id).copied()
    }

    fn get_mut_checked(&mut self, upload_id: &str) -> Result<&mut UploadJob, MediaError> {
        self.jobs.get_mut(upload_id).ok_or(MediaError::NotFound {
            diagnostic_id: "p6.4-upload-not-found",
        })
    }

    /// Queued → Uploading.
    pub fn begin(&mut self, upload_id: &str) -> Result<&UploadJob, MediaError> {
        {
            let job = self.get_mut_checked(upload_id)?;
            if job.state != UploadState::Queued {
                return Err(MediaError::Invalid {
                    diagnostic_id: "p6.4-begin-not-queued",
                });
            }
            job.state = UploadState::Uploading;
            job.progress01 = Some(0.0);
        }
        self.failures.remove(upload_id);
        Ok(self.jobs.get(upload_id).expect("job present"))
    }

    /// Update progress while uploading (clamped to \[0,1\]).
    pub fn set_progress(
        &mut self,
        upload_id: &str,
        progress01: f64,
    ) -> Result<&UploadJob, MediaError> {
        let job = self.get_mut_checked(upload_id)?;
        if job.state != UploadState::Uploading {
            return Err(MediaError::Invalid {
                diagnostic_id: "p6.4-progress-not-uploading",
            });
        }
        let p = progress01.clamp(0.0, 1.0);
        job.progress01 = Some(p);
        Ok(job)
    }

    /// Mark completed with media handle id (no bytes).
    pub fn complete(
        &mut self,
        upload_id: &str,
        media_handle_id: impl Into<String>,
    ) -> Result<&UploadJob, MediaError> {
        let handle = media_handle_id.into().trim().to_owned();
        if handle.is_empty() {
            return Err(MediaError::Invalid {
                diagnostic_id: "p6.4-empty-media-handle",
            });
        }
        {
            let job = self.get_mut_checked(upload_id)?;
            if job.state != UploadState::Uploading {
                return Err(MediaError::Invalid {
                    diagnostic_id: "p6.4-complete-not-uploading",
                });
            }
            job.state = UploadState::Completed;
            job.progress01 = Some(1.0);
            job.media_handle_id = Some(handle);
        }
        self.failures.remove(upload_id);
        Ok(self.jobs.get(upload_id).expect("job present"))
    }

    pub fn fail(
        &mut self,
        upload_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<&UploadJob, MediaError> {
        {
            let job = self.get_mut_checked(upload_id)?;
            if !matches!(job.state, UploadState::Queued | UploadState::Uploading) {
                return Err(MediaError::Invalid {
                    diagnostic_id: "p6.4-fail-invalid-state",
                });
            }
            job.state = UploadState::Failed;
        }
        self.failures.insert(upload_id.to_owned(), diagnostic_id);
        Ok(self.jobs.get(upload_id).expect("job present"))
    }

    pub fn cancel(&mut self, upload_id: &str) -> Result<&UploadJob, MediaError> {
        {
            let job = self.get_mut_checked(upload_id)?;
            if matches!(job.state, UploadState::Completed | UploadState::Cancelled) {
                return Err(MediaError::Invalid {
                    diagnostic_id: "p6.4-cancel-invalid-state",
                });
            }
            job.state = UploadState::Cancelled;
        }
        self.failures.remove(upload_id);
        Ok(self.jobs.get(upload_id).expect("job present"))
    }

    pub fn list(&self) -> Vec<&UploadJob> {
        self.order
            .iter()
            .filter_map(|id| self.jobs.get(id))
            .collect()
    }

    /// Drop terminal jobs (completed/failed/cancelled).
    pub fn prune_terminal(&mut self) -> usize {
        let before = self.jobs.len();
        self.order.retain(|id| {
            self.jobs
                .get(id)
                .map(|j| {
                    !matches!(
                        j.state,
                        UploadState::Completed | UploadState::Failed | UploadState::Cancelled
                    )
                })
                .unwrap_or(false)
        });
        self.jobs.retain(|_, j| {
            !matches!(
                j.state,
                UploadState::Completed | UploadState::Failed | UploadState::Cancelled
            )
        });
        self.failures.retain(|id, _| self.jobs.contains_key(id));
        before.saturating_sub(self.jobs.len())
    }

    /// Cancel active jobs and bump generation (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        for job in self.jobs.values_mut() {
            if matches!(job.state, UploadState::Queued | UploadState::Uploading) {
                job.state = UploadState::Cancelled;
            }
        }
        self.failures.clear();
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.jobs.clear();
        self.failures.clear();
    }
}
