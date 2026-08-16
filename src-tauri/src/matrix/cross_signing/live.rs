//! Desktop re-export of Core cross-signing live projections.

pub use synara_core::app::cross_signing::{
    project_cross_signing_status, project_status, supported_authentication,
    NativeCrossSigningBootstrap, NativeCrossSigningKeyPublication, NativeCrossSigningPrivateFlags,
    NativeCrossSigningPrivateIdentity, NativeCrossSigningReadiness, NativeCrossSigningSetupOutcome,
    NativeCrossSigningSetupResult, NativeCrossSigningStatus, NativeOwnIdentityVerification,
    SupportedBootstrapAuthentication,
};
