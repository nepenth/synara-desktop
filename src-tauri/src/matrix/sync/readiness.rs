//! Product-facing sync readiness model (P4.1).
//!
//! Maps `matrix_sdk_ui::sync_service::State` into privacy-safe phases that align
//! with [`crate::matrix::diagnostics::SyncPhase`] vocabulary. No tokens, no
//! homeserver raw errors, no dual-backend.

use matrix_sdk_ui::sync_service::State as SdkSyncState;
use serde::{Deserialize, Serialize};

use crate::matrix::diagnostics::SyncPhase;

/// High-level product readiness of the Matrix sync owner.
///
/// Distinct from supervisor lifecycle (`Syncing` / `Ready`): this tracks the
/// **SyncService** loop itself after a session is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncReadiness {
    /// No SyncService built yet (or fully torn down).
    Unconfigured,
    /// Service built; not started (SDK Idle).
    Idle,
    /// Sliding sync + encryption sync loops are running.
    Running,
    /// Offline mode (server reachability probe). Optional SDK feature.
    Offline,
    /// Gracefully stopped / terminated; restartable.
    Terminated,
    /// Terminal error; restart required (privacy-safe code only).
    Failed,
}

impl SyncReadiness {
    pub const ALL: &'static [SyncReadiness] = &[
        Self::Unconfigured,
        Self::Idle,
        Self::Running,
        Self::Offline,
        Self::Terminated,
        Self::Failed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unconfigured => "unconfigured",
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Offline => "offline",
            Self::Terminated => "terminated",
            Self::Failed => "failed",
        }
    }

    /// Whether the product may treat live state as flowing (room list / streams).
    pub fn allows_live_projections(self) -> bool {
        matches!(self, Self::Running)
    }

    /// Whether product "sync ready" criteria for the partial path are met.
    ///
    /// First usable connected state = running SyncService (room list may still
    /// be catching up; P4.2 owns list snapshots). Offline/failed are not ready.
    pub fn is_product_ready(self) -> bool {
        matches!(self, Self::Running)
    }

    /// Map into diagnostics [`SyncPhase`] for health snapshots.
    pub fn to_sync_phase(self) -> SyncPhase {
        match self {
            Self::Unconfigured | Self::Idle | Self::Terminated => SyncPhase::Idle,
            Self::Running => SyncPhase::Live,
            Self::Offline => SyncPhase::Reconnecting,
            Self::Failed => SyncPhase::Failed,
        }
    }
}

/// Privacy-safe snapshot of sync readiness (no SDK error payloads).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReadinessSnapshot {
    pub readiness: SyncReadiness,
    pub session_generation: u64,
    pub offline_mode_enabled: bool,
    /// Stable diagnostic code when `readiness == Failed`; never a raw SDK message.
    pub failure_diagnostic_id: Option<&'static str>,
    /// Best-effort sliding-sync preflight verdict (`None` = not yet probed /
    /// probe could not complete). Added 2026-08-10 (P1 capability preflight);
    /// purely informational — never gates sync.
    #[serde(default)]
    pub sliding_sync_capable: Option<bool>,
}

impl SyncReadinessSnapshot {
    pub fn unconfigured(session_generation: u64) -> Self {
        Self {
            readiness: SyncReadiness::Unconfigured,
            session_generation,
            offline_mode_enabled: false,
            failure_diagnostic_id: None,
            sliding_sync_capable: None,
        }
    }

    pub fn is_product_ready(&self) -> bool {
        self.readiness.is_product_ready()
    }

    /// Attach a best-effort sliding-sync capability verdict (informational).
    pub fn with_sliding_sync_capability(mut self, capable: Option<bool>) -> Self {
        self.sliding_sync_capable = capable;
        self
    }
}

/// Map a single SDK SyncService state observation into product readiness.
///
/// Error variants are collapsed to [`SyncReadiness::Failed`] with a fixed
/// diagnostic id — raw error text is never exported.
pub fn readiness_from_sdk_state(state: &SdkSyncState) -> SyncReadiness {
    match state {
        SdkSyncState::Idle => SyncReadiness::Idle,
        SdkSyncState::Running => SyncReadiness::Running,
        SdkSyncState::Terminated => SyncReadiness::Terminated,
        SdkSyncState::Offline => SyncReadiness::Offline,
        SdkSyncState::Error(_) => SyncReadiness::Failed,
    }
}

/// Diagnostic id for a failed SDK state (no raw message).
pub fn failure_diagnostic_from_sdk_state(state: &SdkSyncState) -> Option<&'static str> {
    match state {
        SdkSyncState::Error(_) => Some("p4.1-sync-service-error"),
        _ => None,
    }
}

/// Build a privacy-safe snapshot from an observed SDK state.
pub fn snapshot_from_sdk_state(
    state: &SdkSyncState,
    session_generation: u64,
    offline_mode_enabled: bool,
) -> SyncReadinessSnapshot {
    SyncReadinessSnapshot {
        readiness: readiness_from_sdk_state(state),
        session_generation,
        offline_mode_enabled,
        failure_diagnostic_id: failure_diagnostic_from_sdk_state(state),
        sliding_sync_capable: None,
    }
}
