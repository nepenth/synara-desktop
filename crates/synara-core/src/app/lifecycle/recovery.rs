//! Failed-store recovery policy — **never** auto-deletes (plan §8.3 / P0.7).
//!
//! Surfaces `StoreCorrupt` / `StoreUnavailable` / `StoreLocked` categories and
//! records diagnostics. Explicit wipe is a separate deliberate call to
//! [`super::logout::perform_local_wipe`] / [`super::wipe::wipe_account_store`].

use crate::app::diagnostics::{MatrixMetrics, StoreHealthStatus};
use crate::app::supervisor::{MatrixSupervisor, SupervisorError};
use crate::transport::MatrixIpcErrorCategory;

use super::LifecycleError;

/// Class of store failure observed by open/continuity layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreFailureKind {
    Corrupt,
    Unavailable,
    Locked,
}

impl StoreFailureKind {
    pub const ALL: &'static [StoreFailureKind] = &[Self::Corrupt, Self::Unavailable, Self::Locked];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Corrupt => "corrupt",
            Self::Unavailable => "unavailable",
            Self::Locked => "locked",
        }
    }

    pub fn ipc_category(self) -> MatrixIpcErrorCategory {
        match self {
            Self::Corrupt => MatrixIpcErrorCategory::StoreCorrupt,
            Self::Unavailable => MatrixIpcErrorCategory::StoreUnavailable,
            Self::Locked => MatrixIpcErrorCategory::StoreLocked,
        }
    }

    pub fn store_health_status(self) -> StoreHealthStatus {
        match self {
            Self::Corrupt => StoreHealthStatus::Corrupt,
            Self::Unavailable => StoreHealthStatus::Unavailable,
            Self::Locked => StoreHealthStatus::Locked,
        }
    }
}

/// Observed store failure (privacy-safe).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreFailure {
    pub kind: StoreFailureKind,
    pub diagnostic_id: &'static str,
}

impl StoreFailure {
    pub fn new(kind: StoreFailureKind) -> Self {
        let diagnostic_id = match kind {
            StoreFailureKind::Corrupt => "p2.6-store-corrupt",
            StoreFailureKind::Unavailable => "p2.6-store-unavailable",
            StoreFailureKind::Locked => "p2.6-store-locked",
        };
        Self {
            kind,
            diagnostic_id,
        }
    }

    pub fn with_diagnostic(kind: StoreFailureKind, diagnostic_id: &'static str) -> Self {
        Self {
            kind,
            diagnostic_id,
        }
    }
}

/// Recommended recovery action. **Never** requests automatic wipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAction {
    pub category: MatrixIpcErrorCategory,
    pub diagnostic_id: &'static str,
    pub store_status: StoreHealthStatus,
    /// Hard invariant: always `false`.
    auto_wipe: bool,
}

impl RecoveryAction {
    pub fn requests_wipe(&self) -> bool {
        self.auto_wipe
    }
}

/// Plan recovery for a store failure. Pure: no I/O, never deletes.
pub fn recovery_action_for(failure: &StoreFailure) -> RecoveryAction {
    RecoveryAction {
        category: failure.kind.ipc_category(),
        diagnostic_id: failure.diagnostic_id,
        store_status: failure.kind.store_health_status(),
        auto_wipe: false,
    }
}

/// Apply non-destructive recovery: metrics + optional supervisor fail.
///
/// Does **not** call wipe APIs. Returns the recovery action for product copy.
pub fn apply_store_failure(
    failure: &StoreFailure,
    metrics: Option<&mut MatrixMetrics>,
    supervisor: Option<&mut MatrixSupervisor>,
) -> Result<RecoveryAction, LifecycleError> {
    let action = recovery_action_for(failure);
    debug_assert!(!action.requests_wipe());

    if let Some(m) = metrics {
        m.set_store_status(action.store_status);
        m.record_store_open_failure();
        m.record_error(action.category, Some(action.diagnostic_id));
    }

    if let Some(s) = supervisor {
        // Best-effort: Fail is only legal from certain states; ignore illegal.
        match s.fail(action.category, action.diagnostic_id) {
            Ok(()) => {}
            Err(SupervisorError::Transition(_)) => {}
            Err(e) => {
                return Err(LifecycleError::Supervisor {
                    diagnostic_id: "p2.6-recovery-supervisor-fail",
                    detail: e.to_string(),
                });
            }
        }
    }

    Ok(action)
}

/// Convenience: surface corrupt store without wiping.
pub fn surface_store_corrupt(
    metrics: Option<&mut MatrixMetrics>,
    supervisor: Option<&mut MatrixSupervisor>,
) -> Result<RecoveryAction, LifecycleError> {
    apply_store_failure(
        &StoreFailure::new(StoreFailureKind::Corrupt),
        metrics,
        supervisor,
    )
}

/// Convenience: surface unavailable store without wiping.
pub fn surface_store_unavailable(
    metrics: Option<&mut MatrixMetrics>,
    supervisor: Option<&mut MatrixSupervisor>,
) -> Result<RecoveryAction, LifecycleError> {
    apply_store_failure(
        &StoreFailure::new(StoreFailureKind::Unavailable),
        metrics,
        supervisor,
    )
}
