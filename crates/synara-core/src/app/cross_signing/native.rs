//! Credential-free V-CRYPTO.2 cross-signing presentation DTOs and projector.
//!
//! Live Client setup start lives in `live.rs`. Password UIAA stays desktop.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningKeyPublication {
    Missing,
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningPrivateIdentity {
    Missing,
    Partial,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeOwnIdentityVerification {
    Missing,
    Unverified,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningReadiness {
    Unavailable,
    SetupRequired,
    RecoveryRequired,
    VerificationRequired,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningBootstrap {
    Needed,
    NotNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCrossSigningSetupOutcome {
    Complete,
    AlreadyConfigured,
    AuthenticationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrossSigningSetupResult {
    pub outcome: NativeCrossSigningSetupOutcome,
    pub status: NativeCrossSigningStatus,
}

/// SDK-neutral private-identity flags used by the presentation projector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCrossSigningPrivateFlags {
    pub has_master: bool,
    pub has_self_signing: bool,
    pub has_user_signing: bool,
}

impl NativeCrossSigningPrivateFlags {
    pub fn is_complete(self) -> bool {
        self.has_master && self.has_self_signing && self.has_user_signing
    }

    pub fn has_any(self) -> bool {
        self.has_master || self.has_self_signing || self.has_user_signing
    }
}

pub fn project_cross_signing_status(
    session_generation: u64,
    private_status: Option<NativeCrossSigningPrivateFlags>,
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
        Some(status) if status.has_any() => NativeCrossSigningPrivateIdentity::Partial,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(
        master: bool,
        self_signing: bool,
        user_signing: bool,
    ) -> NativeCrossSigningPrivateFlags {
        NativeCrossSigningPrivateFlags {
            has_master: master,
            has_self_signing: self_signing,
            has_user_signing: user_signing,
        }
    }

    #[test]
    fn projection_distinguishes_setup_recovery_verification_and_ready() {
        assert_eq!(
            project_cross_signing_status(1, Some(flags(false, false, false)), false, false)
                .readiness,
            NativeCrossSigningReadiness::SetupRequired
        );
        assert_eq!(
            project_cross_signing_status(1, Some(flags(true, false, false)), true, false).readiness,
            NativeCrossSigningReadiness::RecoveryRequired
        );
        assert_eq!(
            project_cross_signing_status(1, Some(flags(true, true, true)), true, false).readiness,
            NativeCrossSigningReadiness::VerificationRequired
        );
        assert_eq!(
            project_cross_signing_status(1, Some(flags(true, true, true)), true, true).readiness,
            NativeCrossSigningReadiness::Ready
        );
    }

    #[test]
    fn projection_contains_only_privacy_safe_status_values() {
        let status = project_cross_signing_status(9, Some(flags(true, true, true)), true, true);
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
}
