//! Privacy-filtered Matrix health snapshot (lifecycle / sync / queue / store /
//! tasks / errors). Compatible field vocabulary with desktop diagnostics.

use serde::{Deserialize, Serialize};

use crate::app::supervisor::{FailureInfo, SupervisorSnapshot, SupervisorState};
use crate::task::{TaskKind, TaskSupervisor};
use crate::transport::MatrixIpcErrorCategory;

/// Schema version for [`MatrixHealthSnapshot`] JSON projections.
pub const MATRIX_HEALTH_SCHEMA_VERSION: u32 = 1;

/// High-level sync activity phase (product-neutral; no homeserver details).
///
/// SNC-P1-5a seam: the pure enum lives in `crate::app::sync`; re-export here
/// so every `crate::app::diagnostics::SyncPhase` path keeps resolving.
pub use crate::app::sync::SyncPhase;

/// Store subsystem readiness (no paths, keys, or account identifiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreHealthStatus {
    #[default]
    Unknown,
    Ready,
    Locked,
    Unavailable,
    Corrupt,
    Missing,
}

impl StoreHealthStatus {
    pub const ALL: &'static [StoreHealthStatus] = &[
        Self::Unknown,
        Self::Ready,
        Self::Locked,
        Self::Unavailable,
        Self::Corrupt,
        Self::Missing,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ready => "ready",
            Self::Locked => "locked",
            Self::Unavailable => "unavailable",
            Self::Corrupt => "corrupt",
            Self::Missing => "missing",
        }
    }
}

/// Task-kind counter row (counts only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskKindCounts {
    pub sync: u64,
    pub listener: u64,
    pub upload: u64,
    pub search: u64,
    pub generic: u64,
}

impl TaskKindCounts {
    pub fn set(&mut self, kind: TaskKind, count: u64) {
        match kind {
            TaskKind::Sync => self.sync = count,
            TaskKind::Listener => self.listener = count,
            TaskKind::Upload => self.upload = count,
            TaskKind::Search => self.search = count,
            TaskKind::Generic => self.generic = count,
        }
    }

    pub fn total(&self) -> u64 {
        self.sync
            .saturating_add(self.listener)
            .saturating_add(self.upload)
            .saturating_add(self.search)
            .saturating_add(self.generic)
    }
}

/// Supervised-task aggregate metrics (from [`TaskSupervisor`] counters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHealth {
    pub live_generation: u64,
    pub registered: u64,
    pub running: u64,
    pub spawned_total: u64,
    pub joined_total: u64,
    pub cancelled_requests: u64,
    pub registered_by_kind: TaskKindCounts,
}

impl TaskHealth {
    pub fn from_supervisor(tasks: &TaskSupervisor) -> Self {
        let mut by_kind = TaskKindCounts::default();
        for kind in TaskKind::ALL {
            by_kind.set(*kind, tasks.count_for_kind(*kind) as u64);
        }
        Self {
            live_generation: tasks.live_generation(),
            registered: tasks.registered_count() as u64,
            running: tasks.running_count() as u64,
            spawned_total: tasks.spawned_total(),
            joined_total: tasks.joined_total(),
            cancelled_requests: tasks.cancelled_requests(),
            registered_by_kind: by_kind,
        }
    }
}

/// Bounded queue / stream backpressure metrics (counts only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueHealth {
    /// Current stream/send queue depth observation.
    pub depth: u64,
    /// High-water mark observed this process.
    pub max_depth: u64,
    /// Configured soft max (typically `MAX_STREAM_QUEUE_DEPTH`).
    pub soft_max: u64,
    /// Dropped / rejected messages due to backpressure.
    pub dropped: u64,
    /// Coalesced updates (room-activity style burst folding).
    pub coalesced: u64,
}

/// Store subsystem health (no secrets, no absolute paths).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreHealth {
    pub status: StoreHealthStatus,
    pub state_ready: bool,
    pub crypto_ready: bool,
    pub cache_ready: bool,
    pub media_ready: bool,
    pub open_failures: u64,
}

/// Error category counters (stable IPC categories only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorHealth {
    /// Total classified errors recorded.
    pub total: u64,
    /// Per-category counts aligned with [`MatrixIpcErrorCategory::ALL`] order.
    pub by_category: Vec<CategoryCount>,
    /// Last privacy-safe diagnostic code (never a token or body).
    pub last_diagnostic_id: Option<String>,
    pub last_category: Option<String>,
}

impl Default for ErrorHealth {
    fn default() -> Self {
        Self {
            total: 0,
            by_category: MatrixIpcErrorCategory::ALL
                .iter()
                .map(|c| CategoryCount {
                    category: c.as_str().to_owned(),
                    count: 0,
                })
                .collect(),
            last_diagnostic_id: None,
            last_category: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCount {
    pub category: String,
    pub count: u64,
}

/// Lifecycle slice of the health model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleHealth {
    pub state: String,
    pub session_generation: u64,
    pub has_client: bool,
    pub live_handles: u64,
    pub installed_total: u64,
    pub shutdown_total: u64,
    pub last_failure_category: Option<String>,
    pub last_failure_diagnostic_id: Option<String>,
}

impl LifecycleHealth {
    pub fn from_supervisor_snapshot(snap: &SupervisorSnapshot) -> Self {
        let (cat, diag) = match &snap.last_failure {
            Some(FailureInfo {
                category,
                diagnostic_id,
            }) => (
                Some(category.as_str().to_owned()),
                Some((*diagnostic_id).to_owned()),
            ),
            None => (None, None),
        };
        Self {
            state: snap.state.as_str().to_owned(),
            session_generation: snap.session_generation,
            has_client: snap.has_client,
            live_handles: snap.live_handles as u64,
            // installed/shutdown totals live on the actor; filled by collector.
            installed_total: 0,
            shutdown_total: 0,
            last_failure_category: cat,
            last_failure_diagnostic_id: diag,
        }
    }

    pub fn empty_state() -> Self {
        Self {
            state: SupervisorState::Empty.as_str().to_owned(),
            session_generation: 0,
            has_client: false,
            live_handles: 0,
            installed_total: 0,
            shutdown_total: 0,
            last_failure_category: None,
            last_failure_diagnostic_id: None,
        }
    }
}

/// Sync activity counters (no room/event identities).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncHealth {
    pub phase: SyncPhase,
    pub transition_count: u64,
    pub recovery_requests: u64,
    pub last_duration_ms: Option<u64>,
}

impl Default for SyncHealth {
    fn default() -> Self {
        Self {
            phase: SyncPhase::Idle,
            transition_count: 0,
            recovery_requests: 0,
            last_duration_ms: None,
        }
    }
}

/// Full privacy-filtered Matrix health snapshot.
///
/// **Never** contains tokens, keys, MXIDs, room/event IDs, homeserver URLs,
/// message bodies, ciphertext, or absolute filesystem paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixHealthSnapshot {
    pub schema_version: u32,
    pub lifecycle: LifecycleHealth,
    pub sync: SyncHealth,
    pub queue: QueueHealth,
    pub store: StoreHealth,
    pub tasks: TaskHealth,
    pub errors: ErrorHealth,
}

impl MatrixHealthSnapshot {
    pub fn empty() -> Self {
        Self {
            schema_version: MATRIX_HEALTH_SCHEMA_VERSION,
            lifecycle: LifecycleHealth::empty_state(),
            sync: SyncHealth::default(),
            queue: QueueHealth::default(),
            store: StoreHealth::default(),
            tasks: TaskHealth::default(),
            errors: ErrorHealth::default(),
        }
    }

    /// Rough overall readiness for harness assertions (not a product UX signal).
    pub fn is_session_ready(&self) -> bool {
        self.lifecycle.state == SupervisorState::Ready.as_str()
            && self.lifecycle.has_client
            && self.store.status == StoreHealthStatus::Ready
    }
}
