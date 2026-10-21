//! P8.4 — Cross-signing / identity state foundation (harness).
//!
//! Pure projection plus live status, setup, password UIA, the sign-in
//! bootstrap, identity reset and the post-sign-in verification prompt. **No private keys, recovery secrets, or tokens.** Shells pass
//! the account password only as a method argument.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.4-cross-signing.md`

mod error;
mod identity;
mod live;
mod native;
mod reset;
#[cfg(test)]
mod reset_tests;
#[cfg(test)]
mod setup_tests;
mod verify_prompt;

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
pub use reset::{
    IdentityResetFlow, NativeIdentityResetState, NativeIdentityResetStep,
    IDENTITY_RESET_APPROVAL_PENDING, IDENTITY_RESET_AUTH_UNSUPPORTED, IDENTITY_RESET_CANCELLED,
    IDENTITY_RESET_FAILED, IDENTITY_RESET_NETWORK, IDENTITY_RESET_NOT_PENDING,
    IDENTITY_RESET_PASSWORD_EMPTY, IDENTITY_RESET_PASSWORD_REJECTED,
};
pub use verify_prompt::{
    device_needs_verification, DeviceVerificationPromptArm, NativeDeviceVerificationPrompt,
};

#[cfg(test)]
mod tests;
