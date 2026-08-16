//! P2.6 / P3.5 / P3.6 / P3.8 — Destructive lifecycle + session + remote logout.
//!
//! Harness / foundation only until cutover. Coordinates:
//! - logout path: drop client handle, retire tasks, clear session material hooks
//! - exact-target local wipe of per-account store paths only
//! - failed-store recovery that **never** auto-deletes (P0.7 / plan §8.3)
//! - P3.5: seal access/refresh tokens into host-only session vault after login
//! - P3.6: restore sealed material onto an unauthenticated SDK `Client`
//! - P3.8: remote/server logout flow + recovery UX copy keys (no secrets)
//!
//! Integrates with [`crate::matrix::supervisor`] (`BeginWipe` / `CompleteWipe`,
//! logout transitions), [`crate::matrix::tasks`] (generation retire), and
//! optional [`crate::matrix::diagnostics::MatrixMetrics`].
//!
//! D0.1 composes session persist/restore into production Tauri auth commands.
//! There is **no** live sync product path and **no** dual-backend.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p2.6-destructive-lifecycle.md`
//! - `docs/matrix-rust-sdk/p3.5-refresh-token-persistence.md`
//! - `docs/matrix-rust-sdk/p3.6-session-restore.md`
//! - `docs/matrix-rust-sdk/p3.8-remote-logout.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod session_material;

pub use session_material::{KeyringSessionMaterialRefs, KeyringSessionMaterialVault};
pub use synara_core::app::lifecycle::*;

/// Static marker for link / schema smoke (no network, no Client, no wipe).
pub const MATRIX_LIFECYCLE_MARKER: &str =
    "matrix-lifecycle-p2.6+session-p3.5+p3.6+remote-logout-p3.8";

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
    let flow = RemoteLogoutFlow::new(0);
    debug_assert_eq!(flow.phase(), RemoteLogoutPhase::Idle);
    debug_assert_eq!(
        RecoveryCopyKey::LegacySessionReauthRequired.as_str(),
        "legacy_session_reauth_required"
    );
    debug_assert_eq!(
        MATRIX_LIFECYCLE_MARKER,
        "matrix-lifecycle-p2.6+session-p3.5+p3.6+remote-logout-p3.8"
    );
    MATRIX_LIFECYCLE_MARKER
}

#[cfg(test)]
mod tests;
