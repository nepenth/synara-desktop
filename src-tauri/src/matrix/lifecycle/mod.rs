//! P2.6 / P3.5 — Destructive lifecycle + session secret persistence foundation.
//!
//! Harness / foundation only until cutover. Coordinates:
//! - logout path: drop client handle, retire tasks, clear session material hooks
//! - exact-target local wipe of per-account store paths only
//! - failed-store recovery that **never** auto-deletes (P0.7 / plan §8.3)
//! - P3.5: seal access/refresh tokens into host-only session vault after login
//!
//! Integrates with [`crate::matrix::supervisor`] (`BeginWipe` / `CompleteWipe`,
//! logout transitions), [`crate::matrix::tasks`] (generation retire), and
//! optional [`crate::matrix::diagnostics::MatrixMetrics`].
//!
//! **No** production Tauri Matrix commands, **no** live login/sync product path,
//! **no** dual-backend, **no** `restore_session` (P3.6).
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p2.6-destructive-lifecycle.md`
//! - `docs/matrix-rust-sdk/p3.5-refresh-token-persistence.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod logout;
mod recovery;
mod session_material;
mod session_persist;
mod wipe;

pub use error::LifecycleError;
pub use logout::{perform_local_wipe, perform_logout, LogoutOutcome, WipeOutcome};
pub use recovery::{
    apply_store_failure, recovery_action_for, surface_store_corrupt, surface_store_unavailable,
    RecoveryAction, StoreFailure, StoreFailureKind,
};
pub use session_material::{
    clear_session_material, load_session_material, persist_session_material,
    rotate_persisted_session_tokens, HostMatrixSessionSecrets, InMemorySessionMaterialVault,
    KeyringSessionMaterialRefs, KeyringSessionMaterialVault, SessionMaterial, SessionMaterialId,
    SessionMaterialMeta, SessionMaterialVault, SESSION_ENVELOPE_VERSION, SESSION_KIND_MATRIX,
    SESSION_MATERIAL_SERVICE,
};
pub use session_persist::{
    persist_session_after_login, session_material_from_auth_session, SessionPersistOutcome,
};
pub use wipe::{
    assert_exact_account_root, assert_path_is_wipe_allowed, wipe_account_store, WipeReport,
    WipeTarget, WIPE_TARGET_KIND_ACCOUNT_ROOT,
};

/// Static marker for link / schema smoke (no network, no Client, no wipe).
pub const MATRIX_LIFECYCLE_MARKER: &str = "matrix-lifecycle-p2.6+session-persist-p3.5";

/// Touch lifecycle foundation paths so they remain linked in non-test builds.
pub fn matrix_lifecycle_markers() -> &'static str {
    let _svc = SESSION_MATERIAL_SERVICE;
    let _env_v = SESSION_ENVELOPE_VERSION;
    let _kind = SESSION_KIND_MATRIX;
    let _keyring = KeyringSessionMaterialVault::platform_supported();
    let _kinds = [
        StoreFailureKind::Corrupt,
        StoreFailureKind::Unavailable,
        StoreFailureKind::Locked,
    ];
    let action = recovery_action_for(&StoreFailure::new(StoreFailureKind::Corrupt));
    debug_assert!(!action.requests_wipe());
    debug_assert!(_svc.contains("matrix-session"));
    debug_assert_eq!(_env_v, 1);
    debug_assert_eq!(_kind, "matrix");
    let _ = _keyring;
    debug_assert_eq!(
        MATRIX_LIFECYCLE_MARKER,
        "matrix-lifecycle-p2.6+session-persist-p3.5"
    );
    MATRIX_LIFECYCLE_MARKER
}

#[cfg(test)]
mod tests;
