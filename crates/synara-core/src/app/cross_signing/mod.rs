//! P8.4 — Cross-signing / identity state foundation (harness).
//!
//! Pure projection plus live status, setup, password UIA and the sign-in
//! bootstrap. **No private keys, recovery secrets, or tokens.** Shells pass
//! the account password only as a method argument.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.4-cross-signing.md`

mod error;
mod identity;
mod live;
mod native;
#[cfg(test)]
mod setup_tests;

pub use error::CrossSigningError;
pub use identity::{
    CrossSigningStore, IdentityTrust, LocalCrossSigningKeys, RemoteIdentity, MAX_TRACKED_IDENTITIES,
};
pub use live::{
    authenticate_password, bootstrap_after_login, complete, cross_signing_status_projection,
    crypto_status_projection, project_status, query_own_identity, setup, status,
    supported_authentication, CrossSigningPasswordError, LoginCrossSigningOutcome,
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
