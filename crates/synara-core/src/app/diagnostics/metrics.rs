//! Mutable metrics collector for the Matrix health model.
//!
//! Harness / foundation only until cutover. Callers (future supervisor bridge,
//! task registry, stream queues) record privacy-safe counters; never attach
//! tokens, MXIDs, or free-form error strings.

use crate::app::supervisor::{MatrixSupervisor, SupervisorSnapshot};
use crate::task::TaskSupervisor;
use crate::transport::{MatrixIpcErrorCategory, MAX_STREAM_QUEUE_DEPTH};

use super::health::{
    ErrorHealth, LifecycleHealth, MatrixHealthSnapshot, QueueHealth, StoreHealth,
    StoreHealthStatus, SyncHealth, SyncPhase, TaskHealth, MATRIX_HEALTH_SCHEMA_VERSION,
};
use super::redact::safe_diagnostic_label;

/// In-process metrics accumulator for Matrix lifecycle health.
///
/// Threading: not synchronized; own under the single Matrix supervisor / harness
/// owner. Clone for snapshots via [`Self::snapshot`].
#[derive(Debug, Clone)]
pub struct MatrixMetrics {
    lifecycle: LifecycleHealth,
    sync: SyncHealth,
    queue: QueueHealth,
    store: StoreHealth,
    tasks: TaskHealth,
    errors: ErrorHealth,
    /// Parallel array matching [`MatrixIpcErrorCategory::ALL`] order.
    error_counts: Vec<u64>,
}

impl Default for MatrixMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MatrixMetrics {
    pub fn new() -> Self {
        let n = MatrixIpcErrorCategory::ALL.len();
        Self {
            lifecycle: LifecycleHealth::empty_state(),
            sync: SyncHealth::default(),
            queue: QueueHealth {
                depth: 0,
                max_depth: 0,
                soft_max: MAX_STREAM_QUEUE_DEPTH as u64,
                dropped: 0,
                coalesced: 0,
            },
            store: StoreHealth::default(),
            tasks: TaskHealth::default(),
            errors: ErrorHealth::default(),
            error_counts: vec![0; n],
        }
    }

    /// Replace lifecycle slice from a supervisor read-only snapshot.
    pub fn observe_supervisor_snapshot(&mut self, snap: &SupervisorSnapshot) {
        let mut life = LifecycleHealth::from_supervisor_snapshot(snap);
        // Preserve installed/shutdown if we already tracked them; snapshot
        // constructor zeros those — fill from last known when available.
        life.installed_total = self.lifecycle.installed_total;
        life.shutdown_total = self.lifecycle.shutdown_total;
        self.lifecycle = life;
    }

    /// Observe full supervisor actor (includes install/shutdown totals).
    pub fn observe_supervisor(&mut self, actor: &MatrixSupervisor) {
        let snap = actor.snapshot();
        let mut life = LifecycleHealth::from_supervisor_snapshot(&snap);
        life.installed_total = actor.installed_total();
        life.shutdown_total = actor.shutdown_total();
        self.lifecycle = life;
    }

    /// Pull task registry counters into the health model (P2.4 export path).
    pub fn observe_tasks(&mut self, tasks: &TaskSupervisor) {
        self.tasks = TaskHealth::from_supervisor(tasks);
    }

    pub fn set_sync_phase(&mut self, phase: SyncPhase) {
        if self.sync.phase != phase {
            self.sync.transition_count = self.sync.transition_count.saturating_add(1);
            self.sync.phase = phase;
        }
    }

    pub fn record_sync_recovery_request(&mut self) {
        self.sync.recovery_requests = self.sync.recovery_requests.saturating_add(1);
    }

    pub fn record_sync_duration_ms(&mut self, duration_ms: u64) {
        self.sync.last_duration_ms = Some(duration_ms);
    }

    pub fn observe_queue_depth(&mut self, depth: u64) {
        self.queue.depth = depth;
        if depth > self.queue.max_depth {
            self.queue.max_depth = depth;
        }
    }

    pub fn record_queue_dropped(&mut self, n: u64) {
        self.queue.dropped = self.queue.dropped.saturating_add(n);
    }

    pub fn record_queue_coalesced(&mut self, n: u64) {
        self.queue.coalesced = self.queue.coalesced.saturating_add(n);
    }

    pub fn set_store_status(&mut self, status: StoreHealthStatus) {
        self.store.status = status;
    }

    pub fn set_store_readiness(
        &mut self,
        state_ready: bool,
        crypto_ready: bool,
        cache_ready: bool,
        media_ready: bool,
    ) {
        self.store.state_ready = state_ready;
        self.store.crypto_ready = crypto_ready;
        self.store.cache_ready = cache_ready;
        self.store.media_ready = media_ready;
        if state_ready && crypto_ready && self.store.status == StoreHealthStatus::Unknown {
            self.store.status = StoreHealthStatus::Ready;
        }
    }

    pub fn record_store_open_failure(&mut self) {
        self.store.open_failures = self.store.open_failures.saturating_add(1);
    }

    /// Record a classified error. `diagnostic_id` must already be privacy-safe
    /// (static code). Free-form secrets are rejected and ignored.
    pub fn record_error(&mut self, category: MatrixIpcErrorCategory, diagnostic_id: Option<&str>) {
        self.errors.total = self.errors.total.saturating_add(1);
        if let Some(idx) = category_index(category) {
            self.error_counts[idx] = self.error_counts[idx].saturating_add(1);
        }
        self.errors.last_category = Some(category.as_str().to_owned());
        if let Some(id) = diagnostic_id {
            if let Some(safe) = safe_diagnostic_label(id) {
                self.errors.last_diagnostic_id = Some(safe);
            }
            // Unsafe diagnostic ids are dropped (not stored, not echoed).
        }
        self.rebuild_error_by_category();
    }

    fn rebuild_error_by_category(&mut self) {
        self.errors.by_category = MatrixIpcErrorCategory::ALL
            .iter()
            .enumerate()
            .map(|(i, c)| super::health::CategoryCount {
                category: c.as_str().to_owned(),
                count: self.error_counts.get(i).copied().unwrap_or(0),
            })
            .collect();
    }

    /// Immutable privacy-filtered snapshot for diagnostics export / tests.
    pub fn snapshot(&self) -> MatrixHealthSnapshot {
        MatrixHealthSnapshot {
            schema_version: MATRIX_HEALTH_SCHEMA_VERSION,
            lifecycle: self.lifecycle.clone(),
            sync: self.sync.clone(),
            queue: self.queue,
            store: self.store,
            tasks: self.tasks,
            errors: self.errors.clone(),
        }
    }
}

fn category_index(category: MatrixIpcErrorCategory) -> Option<usize> {
    MatrixIpcErrorCategory::ALL
        .iter()
        .position(|c| *c == category)
}
