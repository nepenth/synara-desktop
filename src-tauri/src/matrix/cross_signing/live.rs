//! Live V-CRYPTO.2 cross-signing status and setup projections.
//!
//! Private cross-signing material remains inside matrix-sdk's encrypted crypto
//! store. This module only exposes presence/readiness enums.

use matrix_sdk::{
    encryption::CrossSigningStatus,
    ruma::api::client::uiaa::{AuthType, UiaaInfo},
};

pub use synara_core::app::cross_signing::{
    project_cross_signing_status, NativeCrossSigningBootstrap, NativeCrossSigningKeyPublication,
    NativeCrossSigningPrivateFlags, NativeCrossSigningPrivateIdentity, NativeCrossSigningReadiness,
    NativeCrossSigningSetupOutcome, NativeCrossSigningSetupResult, NativeCrossSigningStatus,
    NativeOwnIdentityVerification,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedBootstrapAuthentication {
    Password,
    Dummy,
}

pub fn project_status(
    session_generation: u64,
    private_status: Option<&CrossSigningStatus>,
    own_identity_published: bool,
    own_identity_verified: bool,
) -> NativeCrossSigningStatus {
    let flags = private_status.map(|status| NativeCrossSigningPrivateFlags {
        has_master: status.has_master,
        has_self_signing: status.has_self_signing,
        has_user_signing: status.has_user_signing,
    });
    project_cross_signing_status(
        session_generation,
        flags,
        own_identity_published,
        own_identity_verified,
    )
}

pub fn supported_authentication(info: &UiaaInfo) -> Option<SupportedBootstrapAuthentication> {
    let stage_is_available = |auth_type: &AuthType| {
        info.flows.iter().any(|flow| {
            flow.stages
                .iter()
                .all(|stage| info.completed.contains(stage) || stage == auth_type)
        })
    };

    if stage_is_available(&AuthType::Dummy) {
        Some(SupportedBootstrapAuthentication::Dummy)
    } else if stage_is_available(&AuthType::Password) {
        Some(SupportedBootstrapAuthentication::Password)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use matrix_sdk::ruma::api::client::uiaa::{AuthFlow, AuthType, UiaaInfo};

    use super::{supported_authentication, SupportedBootstrapAuthentication};

    #[test]
    fn authentication_projection_prefers_automatic_dummy_then_password() {
        let info = UiaaInfo::new(vec![
            AuthFlow::new(vec![AuthType::Password]),
            AuthFlow::new(vec![AuthType::Dummy]),
        ]);
        assert_eq!(
            supported_authentication(&info),
            Some(SupportedBootstrapAuthentication::Dummy)
        );

        let password = UiaaInfo::new(vec![AuthFlow::new(vec![AuthType::Password])]);
        assert_eq!(
            supported_authentication(&password),
            Some(SupportedBootstrapAuthentication::Password)
        );
    }
}
