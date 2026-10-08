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

mod capability;
mod error;
mod readiness;
mod reconnect;
mod service;
mod sync_phase;
mod wake;

pub use capability::{probe_sliding_sync, server_supports_sliding_sync};
pub use error::SyncError;
pub use readiness::{
    failure_diagnostic_from_sdk_state, readiness_from_sdk_state, snapshot_from_sdk_state,
    CommandGate, SyncReadiness, SyncReadinessSnapshot, SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID,
    SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
};
pub use reconnect::{decide_reconnect, is_restartable, ReconnectAction, SyncIntent};
pub use service::{
    assert_generation, build_sync_service, readiness_of, unconfigured_snapshot, SyncServiceConfig,
    SyncServiceOwner,
};
pub use sync_phase::SyncPhase;
pub use wake::{recover_cooldown_active, suspend_detected, RECOVER_COOLDOWN, SUSPEND_WALL_SKEW};
