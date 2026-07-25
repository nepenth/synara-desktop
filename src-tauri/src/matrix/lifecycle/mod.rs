//! P2.6 — Destructive lifecycle operations (logout, local wipe, recovery).
//!
//! Harness / foundation only until cutover. Coordinates:
//! - logout path: drop client handle, retire tasks, clear session material hooks
//! - exact-target local wipe of per-account store paths only
//! - failed-store recovery that **never** auto-deletes (P0.7 / plan §8.3)
//!
//! Integrates with [`crate::matrix::supervisor`] (`BeginWipe` / `CompleteWipe`,
//! logout transitions), [`crate::matrix::tasks`] (generation retire), and
//! optional [`crate::matrix::diagnostics::MatrixMetrics`].
//!
//! **No** production Tauri Matrix commands, **no** live login/sync product path,
//! **no** dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.6-destructive-lifecycle.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod logout;
mod recovery;
mod session_material;
mod wipe;

pub use error::LifecycleError;
pub use logout::{perform_local_wipe, perform_logout, LogoutOutcome, WipeOutcome};
pub use recovery::{
    apply_store_failure, recovery_action_for, surface_store_corrupt, surface_store_unavailable,
    RecoveryAction, StoreFailure, StoreFailureKind,
};
pub use session_material::{
    clear_session_material, InMemorySessionMaterialVault, SessionMaterial, SessionMaterialId,
    SessionMaterialVault, SESSION_MATERIAL_SERVICE,
};
pub use wipe::{
    assert_exact_account_root, assert_path_is_wipe_allowed, wipe_account_store, WipeReport,
    WipeTarget,
};

/// Static marker for link / schema smoke (no network, no Client, no wipe).
pub const MATRIX_LIFECYCLE_MARKER: &str = "matrix-destructive-lifecycle-p2.6";

/// Touch lifecycle foundation paths so they remain linked in non-test builds.
pub fn matrix_lifecycle_markers() -> &'static str {
    let _svc = SESSION_MATERIAL_SERVICE;
    let _kinds = [
        StoreFailureKind::Corrupt,
        StoreFailureKind::Unavailable,
        StoreFailureKind::Locked,
    ];
    let action = recovery_action_for(&StoreFailure::new(StoreFailureKind::Corrupt));
    debug_assert!(!action.requests_wipe());
    debug_assert!(_svc.contains("matrix-session"));
    debug_assert_eq!(MATRIX_LIFECYCLE_MARKER, "matrix-destructive-lifecycle-p2.6");
    MATRIX_LIFECYCLE_MARKER
}

#[cfg(test)]
mod tests;
