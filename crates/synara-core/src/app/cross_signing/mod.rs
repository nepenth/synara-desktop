//! P8.4 — Cross-signing / identity state foundation (harness).
//!
//! Pure projection plus live status/setup start. **No private keys, recovery
//! secrets, or tokens.** Password UIAA stays in the desktop shell.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.4-cross-signing.md`

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

#[cfg(test)]
mod tests;
