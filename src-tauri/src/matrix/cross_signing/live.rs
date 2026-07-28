//! Live V-CRYPTO.2 cross-signing status and setup projections.
//!
//! Private cross-signing material remains inside matrix-sdk's encrypted crypto
//! store. This module only exposes presence/readiness enums.

use matrix_sdk::{
    encryption::CrossSigningStatus,
    ruma::api::client::uiaa::{AuthType, UiaaInfo},
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningKeyPublication {
    Missing,
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningPrivateIdentity {
    Missing,
    Partial,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeOwnIdentityVerification {
    Missing,
    Unverified,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningReadiness {
    Unavailable,
    SetupRequired,
    RecoveryRequired,
    VerificationRequired,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningBootstrap {
    Needed,
    NotNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrossSigningStatus {
    pub session_generation: u64,
    pub readiness: NativeCrossSigningReadiness,
    pub master_signing: NativeCrossSigningKeyPublication,
    pub self_signing: NativeCrossSigningKeyPublication,
    pub user_signing: NativeCrossSigningKeyPublication,
    pub private_identity: NativeCrossSigningPrivateIdentity,
    pub own_identity_verification: NativeOwnIdentityVerification,
    pub bootstrap: NativeCrossSigningBootstrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningSetupOutcome {
    Complete,
    AlreadyConfigured,
    AuthenticationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrossSigningSetupResult {
    pub outcome: NativeCrossSigningSetupOutcome,
    pub status: NativeCrossSigningStatus,
}

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
    let publication = if own_identity_published {
        NativeCrossSigningKeyPublication::Published
    } else {
        NativeCrossSigningKeyPublication::Missing
    };
    let private_identity = match private_status {
        None => NativeCrossSigningPrivateIdentity::Missing,
        Some(status) if status.is_complete() => NativeCrossSigningPrivateIdentity::Complete,
        Some(status) if status.has_master || status.has_self_signing || status.has_user_signing => {
            NativeCrossSigningPrivateIdentity::Partial
        }
        Some(_) => NativeCrossSigningPrivateIdentity::Missing,
    };
    let own_identity_verification = if !own_identity_published {
        NativeOwnIdentityVerification::Missing
    } else if own_identity_verified {
        NativeOwnIdentityVerification::Verified
    } else {
        NativeOwnIdentityVerification::Unverified
    };
    let readiness = match (
        private_status,
        own_identity_published,
        own_identity_verified,
    ) {
        (None, _, _) => NativeCrossSigningReadiness::Unavailable,
        (Some(_), false, _) => NativeCrossSigningReadiness::SetupRequired,
        (Some(status), true, _) if !status.is_complete() => {
            NativeCrossSigningReadiness::RecoveryRequired
        }
        (Some(_), true, false) => NativeCrossSigningReadiness::VerificationRequired,
        (Some(_), true, true) => NativeCrossSigningReadiness::Ready,
    };

    NativeCrossSigningStatus {
        session_generation,
        readiness,
        master_signing: publication,
        self_signing: publication,
        user_signing: publication,
        private_identity,
        own_identity_verification,
        bootstrap: if private_status.is_some() && !own_identity_published {
            NativeCrossSigningBootstrap::Needed
        } else {
            NativeCrossSigningBootstrap::NotNeeded
        },
    }
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
    use matrix_sdk::{
        encryption::CrossSigningStatus,
        ruma::api::client::uiaa::{AuthFlow, AuthType, UiaaInfo},
    };

    use super::{
        project_status, supported_authentication, NativeCrossSigningPrivateIdentity,
        NativeCrossSigningReadiness, NativeOwnIdentityVerification,
        SupportedBootstrapAuthentication,
    };

    #[test]
    fn projection_distinguishes_setup_recovery_verification_and_ready() {
        let empty = CrossSigningStatus {
            has_master: false,
            has_self_signing: false,
            has_user_signing: false,
        };
        let partial = CrossSigningStatus {
            has_master: true,
            has_self_signing: false,
            has_user_signing: false,
        };
        let complete = CrossSigningStatus {
            has_master: true,
            has_self_signing: true,
            has_user_signing: true,
        };

        assert_eq!(
            project_status(1, Some(&empty), false, false).readiness,
            NativeCrossSigningReadiness::SetupRequired
        );
        assert_eq!(
            project_status(1, Some(&partial), true, false).readiness,
            NativeCrossSigningReadiness::RecoveryRequired
        );
        assert_eq!(
            project_status(1, Some(&complete), true, false).readiness,
            NativeCrossSigningReadiness::VerificationRequired
        );
        assert_eq!(
            project_status(1, Some(&complete), true, true).readiness,
            NativeCrossSigningReadiness::Ready
        );
    }

    #[test]
    fn projection_contains_only_privacy_safe_status_values() {
        let status = project_status(
            9,
            Some(&CrossSigningStatus {
                has_master: true,
                has_self_signing: true,
                has_user_signing: true,
            }),
            true,
            true,
        );
        assert_eq!(
            status.private_identity,
            NativeCrossSigningPrivateIdentity::Complete
        );
        assert_eq!(
            status.own_identity_verification,
            NativeOwnIdentityVerification::Verified
        );

        let json = serde_json::to_string(&status).unwrap().to_ascii_lowercase();
        for forbidden in [
            "access_token",
            "refresh_token",
            "recovery_key",
            "private_key",
            "ciphertext",
            "passphrase",
            "password",
        ] {
            assert!(!json.contains(forbidden));
        }
    }

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
