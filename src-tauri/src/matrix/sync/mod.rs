//! P4.1 — Sync service readiness / reconnect model (src-tauri adapter module).
//!
//! SNC-P1-5a: the sync logic now lives in the shared native core at
//! `crates/synara-core/src/app/sync`. This module keeps every
//! `crate::matrix::sync::…` path (SyncServiceOwner, probe_sliding_sync,
//! server_supports_sliding_sync, SyncError, SyncReadiness, ReconnectAction,
//! SyncIntent, SyncWatchService, …) resolving with
//! **identical behavior** by re-exporting the core items.
//!
//! `tests.rs` stays here (adapter side) because it also imports the
//! src-tauri-only `crate::matrix::client_builder` and
//! the shared `SyncPhase` owner vocabulary (re-exported from the
//! core seam via diagnostics/health.rs).
//!
//! **Harness / foundation only until cutover.** No production Tauri Matrix
//! commands, no room-list deltas (P4.2), no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.1-sync-readiness.md`

pub use synara_core::app::sync::{
    assert_generation, build_sync_service, decide_reconnect, failure_diagnostic_from_sdk_state,
    is_restartable, probe_sliding_sync, readiness_from_sdk_state, readiness_of,
    recover_cooldown_active, server_supports_sliding_sync, snapshot_from_sdk_state,
    suspend_detected, unconfigured_snapshot, ReconnectAction, SyncError, SyncIntent, SyncReadiness,
    SyncReadinessSnapshot, SyncServiceConfig, SyncServiceOwner, RECOVER_COOLDOWN,
    SUSPEND_WALL_SKEW,
};

#[cfg(test)]
mod tests;
