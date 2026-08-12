//! P4.1 — Sync service readiness / reconnect model (shared native core).
//!
//! Owns the product mapping around `matrix_sdk_ui::sync_service::SyncService`:
//! - privacy-safe readiness phases aligned with diagnostics [`SyncPhase`]
//! - pure reconnect decision table (start / stop / restart)
//! - session-generation-stamped owner for one authenticated client
//!
//! SNC-P1-5a: moved from src-tauri `matrix/sync` into the shared core
//! (`crates/synara-core/src/app/sync`). src-tauri's `matrix/sync/mod.rs` is
//! now an adapter that re-exports this module; the [`SyncPhase`] seam lives in
//! [`sync_phase`] and is re-exported by src-tauri diagnostics/health.rs.
//!
//! **Harness / foundation only until cutover.** No production Tauri Matrix
//! commands, no room-list deltas (P4.2), no dual-backend, no JS sync.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.1-sync-readiness.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod capability;
mod error;
mod readiness;
mod reconnect;
mod service;
mod sync_phase;

pub use capability::{probe_sliding_sync, server_supports_sliding_sync};
pub use error::SyncError;
pub use readiness::{
    failure_diagnostic_from_sdk_state, readiness_from_sdk_state, snapshot_from_sdk_state,
    SyncReadiness, SyncReadinessSnapshot, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
};
pub use reconnect::{decide_reconnect, is_restartable, ReconnectAction, SyncIntent};
pub use service::{
    assert_generation, build_sync_service, readiness_of, unconfigured_snapshot, SyncServiceConfig,
    SyncServiceOwner,
};
pub use sync_phase::SyncPhase;

/// Static marker for link / schema smoke (no network, no Client).
pub const MATRIX_SYNC_MARKER: &str = "matrix-sync-readiness-p4.1";

/// Touch sync readiness paths so the foundation remains linked in non-test builds.
pub fn matrix_sync_markers() -> &'static str {
    let _phases = SyncReadiness::ALL.len();
    let _idle = SyncReadiness::Idle.as_str();
    let _ready = SyncReadiness::Running.is_product_ready();
    let _action = decide_reconnect(SyncReadiness::Failed, SyncIntent::Recover);
    let _cfg = SyncServiceConfig::default();
    debug_assert_eq!(_phases, 6);
    debug_assert_eq!(_idle, "idle");
    debug_assert!(_ready);
    debug_assert_eq!(_action, ReconnectAction::Restart);
    debug_assert!(_cfg.offline_mode);
    debug_assert_eq!(MATRIX_SYNC_MARKER, "matrix-sync-readiness-p4.1");
    MATRIX_SYNC_MARKER
}
