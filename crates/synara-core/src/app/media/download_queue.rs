//! Media download / local-delivery job queue (P7.2 harness foundation).
//!
//! Tracks thumbnail / original / avatar fetch intents with local **handle**
//! ids only — **never** file bytes or ciphertext. No SDK media network,
//! no dual-backend, no tokens in errors.

use std::collections::HashMap;

use crate::dto::RoomId;

use super::error::MediaError;

/// Soft cap on concurrent active (queued+fetching) downloads.
pub const MAX_ACTIVE_DOWNLOADS: usize = 32;

/// Soft cap on tracked download jobs (including terminal until pruned).
pub const MAX_TRACKED_DOWNLOADS: usize = 512;

/// Soft cap on media source / handle id length (chars).
pub const MAX_MEDIA_ID_CHARS: usize = 2_048;

/// Opaque local download job id.
pub type DownloadId = String;

/// What the host is fetching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DownloadKind {
    Thumbnail,
    Original,
    Avatar,
}

impl DownloadKind {
    pub const ALL: &'static [DownloadKind] = &[Self::Thumbnail, Self::Original, Self::Avatar];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::Original => "original",
            Self::Avatar => "avatar",
        }
    }
}

/// Download job lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DownloadState {
    Queued,
    Fetching,
    Ready,
    Failed,
    Cancelled,
}

impl DownloadState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Ready | Self::Failed | Self::Cancelled)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Fetching => "fetching",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One media download / local-delivery job (metadata only).
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadJob {
    pub download_id: DownloadId,
    pub session_generation: u64,
    /// Source media id / mxc or product handle — string only, never bytes.
    pub source_media_id: String,
    pub kind: DownloadKind,
    pub room_id: Option<RoomId>,
    pub state: DownloadState,
    /// Progress in \[0, 1\] while Fetching.
    pub progress01: Option<f32>,
    /// Local delivery handle id when Ready (path key / cache key — not raw bytes).
    pub local_handle_id: Option<String>,
    /// Privacy-safe failure diagnostic when Failed.
    pub failure_diagnostic_id: Option<&'static str>,
}

impl DownloadJob {
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

/// Session-generation-stamped download queue.
#[derive(Debug, Default)]
pub struct DownloadQueue {
    session_generation: u64,
    order: Vec<DownloadId>,
    jobs: HashMap<DownloadId, DownloadJob>,
    next_seq: u64,
}

impl DownloadQueue {
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

    pub fn active_count(&self) -> usize {
        self.jobs
            .values()
            .filter(|j| matches!(j.state, DownloadState::Queued | DownloadState::Fetching))
            .count()
    }

    fn alloc_id(&mut self) -> DownloadId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("download-{}", self.next_seq)
    }

    /// Enqueue a media download (source id only — no bytes).
    pub fn enqueue(
        &mut self,
        source_media_id: impl Into<String>,
        kind: DownloadKind,
        room_id: Option<RoomId>,
    ) -> Result<&DownloadJob, MediaError> {
        let source_media_id = source_media_id.into().trim().to_owned();
        validate_media_id(&source_media_id)?;
        if let Some(ref room) = room_id {
            validate_room_id(room)?;
        }
        if self.active_count() >= MAX_ACTIVE_DOWNLOADS {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-active-download-cap",
            });
        }
        if self.jobs.len() >= MAX_TRACKED_DOWNLOADS {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-tracked-download-cap",
            });
        }

        let download_id = self.alloc_id();
        let job = DownloadJob {
            download_id: download_id.clone(),
            session_generation: self.session_generation,
            source_media_id,
            kind,
            room_id,
            state: DownloadState::Queued,
            progress01: None,
            local_handle_id: None,
            failure_diagnostic_id: None,
        };
        self.order.push(download_id.clone());
        self.jobs.insert(download_id.clone(), job);
        Ok(self.jobs.get(&download_id).expect("just inserted"))
    }

    pub fn get(&self, download_id: &str) -> Option<&DownloadJob> {
        self.jobs.get(download_id)
    }

    fn get_mut_checked(&mut self, download_id: &str) -> Result<&mut DownloadJob, MediaError> {
        let job = self.jobs.get_mut(download_id).ok_or(MediaError::NotFound {
            diagnostic_id: "p7.2-download-not-found",
        })?;
        if job.session_generation != self.session_generation {
            return Err(MediaError::StaleGeneration {
                diagnostic_id: "p7.2-stale-generation",
                expected: self.session_generation,
                observed: job.session_generation,
            });
        }
        Ok(job)
    }

    /// Queued → Fetching.
    pub fn begin(&mut self, download_id: &str) -> Result<&DownloadJob, MediaError> {
        let job = self.get_mut_checked(download_id)?;
        if job.state != DownloadState::Queued {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-begin-invalid-state",
            });
        }
        job.state = DownloadState::Fetching;
        job.progress01 = Some(0.0);
        Ok(job)
    }

    pub fn set_progress(
        &mut self,
        download_id: &str,
        progress01: f32,
    ) -> Result<&DownloadJob, MediaError> {
        if !(0.0..=1.0).contains(&progress01) || progress01.is_nan() {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-invalid-progress",
            });
        }
        let job = self.get_mut_checked(download_id)?;
        if job.state != DownloadState::Fetching {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-progress-invalid-state",
            });
        }
        job.progress01 = Some(progress01);
        Ok(job)
    }

    /// Mark Ready with a local delivery handle (path key / cache key only).
    pub fn complete(
        &mut self,
        download_id: &str,
        local_handle_id: impl Into<String>,
    ) -> Result<&DownloadJob, MediaError> {
        let local_handle_id = local_handle_id.into().trim().to_owned();
        if local_handle_id.is_empty() || local_handle_id.chars().count() > MAX_MEDIA_ID_CHARS {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-invalid-local-handle",
            });
        }
        // Forbid schemes that embed payload in the handle string.
        let lower = local_handle_id.to_ascii_lowercase();
        if lower.starts_with("data:") || lower.starts_with("javascript:") {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-forbidden-handle-scheme",
            });
        }
        let job = self.get_mut_checked(download_id)?;
        if job.state != DownloadState::Fetching {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-complete-invalid-state",
            });
        }
        job.state = DownloadState::Ready;
        job.progress01 = Some(1.0);
        job.local_handle_id = Some(local_handle_id);
        job.failure_diagnostic_id = None;
        Ok(job)
    }

    pub fn fail(
        &mut self,
        download_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<&DownloadJob, MediaError> {
        let job = self.get_mut_checked(download_id)?;
        if job.state != DownloadState::Fetching {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-fail-invalid-state",
            });
        }
        job.state = DownloadState::Failed;
        job.failure_diagnostic_id = Some(diagnostic_id);
        Ok(job)
    }

    pub fn cancel(&mut self, download_id: &str) -> Result<&DownloadJob, MediaError> {
        let job = self.get_mut_checked(download_id)?;
        if matches!(job.state, DownloadState::Ready | DownloadState::Cancelled) {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-cancel-invalid-state",
            });
        }
        job.state = DownloadState::Cancelled;
        job.failure_diagnostic_id = None;
        Ok(job)
    }

    /// Failed → Queued for retry.
    pub fn retry(&mut self, download_id: &str) -> Result<&DownloadJob, MediaError> {
        // Capacity check before exclusive borrow of the job.
        let state = self
            .jobs
            .get(download_id)
            .ok_or(MediaError::NotFound {
                diagnostic_id: "p7.2-download-not-found",
            })?
            .state;
        if state != DownloadState::Failed {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-retry-not-failed",
            });
        }
        if self.active_count() >= MAX_ACTIVE_DOWNLOADS {
            return Err(MediaError::Invalid {
                diagnostic_id: "p7.2-active-download-cap",
            });
        }
        let job = self.get_mut_checked(download_id)?;
        job.state = DownloadState::Queued;
        job.progress01 = None;
        job.local_handle_id = None;
        job.failure_diagnostic_id = None;
        Ok(job)
    }

    pub fn list(&self) -> Vec<&DownloadJob> {
        self.order
            .iter()
            .filter_map(|id| self.jobs.get(id))
            .collect()
    }

    pub fn list_active(&self) -> Vec<&DownloadJob> {
        self.list()
            .into_iter()
            .filter(|j| !j.is_terminal())
            .collect()
    }

    pub fn prune_terminal(&mut self) -> usize {
        let before = self.jobs.len();
        self.order
            .retain(|id| self.jobs.get(id).map(|j| !j.is_terminal()).unwrap_or(false));
        self.jobs.retain(|_, j| !j.is_terminal());
        before.saturating_sub(self.jobs.len())
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.jobs.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
        self.next_seq = 0;
    }
}

fn validate_media_id(id: &str) -> Result<(), MediaError> {
    if id.is_empty() {
        return Err(MediaError::Invalid {
            diagnostic_id: "p7.2-empty-media-id",
        });
    }
    if id.chars().count() > MAX_MEDIA_ID_CHARS {
        return Err(MediaError::Invalid {
            diagnostic_id: "p7.2-media-id-cap",
        });
    }
    let lower = id.to_ascii_lowercase();
    if lower.starts_with("data:") || lower.starts_with("javascript:") {
        return Err(MediaError::Invalid {
            diagnostic_id: "p7.2-forbidden-media-scheme",
        });
    }
    if lower.contains("access_token") || lower.contains("refresh_token") {
        return Err(MediaError::Invalid {
            diagnostic_id: "p7.2-forbidden-media-id",
        });
    }
    Ok(())
}

fn validate_room_id(room: &str) -> Result<(), MediaError> {
    if room.is_empty() || !room.starts_with('!') {
        return Err(MediaError::Invalid {
            diagnostic_id: "p7.2-invalid-room-id",
        });
    }
    Ok(())
}
