//! P8.4 — Cross-signing / identity state foundation (harness).
//!
//! Pure projection plus live status/setup start. **No private keys, recovery
//! secrets, or tokens.** Password UIAA stays in the desktop shell.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.4-cross-signing.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod identity;
mod live;
mod native;

pub use error::CrossSigningError;
pub use identity::{
    CrossSigningStore, IdentityTrust, LocalCrossSigningKeys, RemoteIdentity, MAX_TRACKED_IDENTITIES,
};
pub use live::{
    complete, project_status, query_own_identity, setup, status, supported_authentication,
    SupportedBootstrapAuthentication,
};
pub use native::{
    project_cross_signing_status, NativeCrossSigningBootstrap, NativeCrossSigningKeyPublication,
    NativeCrossSigningPrivateFlags, NativeCrossSigningPrivateIdentity, NativeCrossSigningReadiness,
    NativeCrossSigningSetupOutcome, NativeCrossSigningSetupResult, NativeCrossSigningStatus,
    NativeOwnIdentityVerification,
};

/// Static marker for link / schema smoke.
pub const MATRIX_CROSS_SIGNING_MARKER: &str = "matrix-cross-signing-p8.4";

/// Touch cross-signing paths so they remain linked in non-test builds.
pub fn matrix_cross_signing_markers() -> &'static str {
    let store = CrossSigningStore::new(0);
    debug_assert!(store.is_empty());
    debug_assert_eq!(store.tracked_count(), 0);
    debug_assert!(store.needs_attention());
    debug_assert_eq!(MATRIX_CROSS_SIGNING_MARKER, "matrix-cross-signing-p8.4");
    MATRIX_CROSS_SIGNING_MARKER
}

#[cfg(test)]
mod tests;
