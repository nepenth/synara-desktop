//! S5 "Verify this device" prompt, shown once after a password sign-in on a
//! device the account's existing identity has not verified.
//!
//! Core arms it for the session the sign-in opened, with whether the sign-in
//! bootstrap created the account's identity on this device (a brand-new
//! account never sees it). Restored sessions are never armed, and "Later"
//! dismisses it for the session; Settings keeps the verification entry.

use matrix_sdk::Client;
use serde::{Deserialize, Serialize};

use super::live::query_own_identity;
use super::LoginCrossSigningOutcome;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDeviceVerificationPrompt {
    pub session_generation: u64,
    pub show: bool,
}

/// What Core remembers about the prompt for the open session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceVerificationPromptArm {
    /// Armed by a sign-in. `identity_created` when that sign-in published the
    /// account's first identity from this device.
    Armed {
        session_generation: u64,
        identity_created: bool,
    },
    /// "Later" was chosen; not shown again for this session.
    Dismissed { session_generation: u64 },
}

impl DeviceVerificationPromptArm {
    pub fn after_sign_in(session_generation: u64, outcome: LoginCrossSigningOutcome) -> Self {
        Self::Armed {
            session_generation,
            identity_created: outcome == LoginCrossSigningOutcome::Created,
        }
    }

    /// Whether the device still has to be checked for this session. Only an
    /// armed prompt from a sign-in that found an existing identity qualifies.
    pub fn may_show(arm: Option<Self>, session_generation: u64) -> bool {
        matches!(
            arm,
            Some(Self::Armed {
                session_generation: armed,
                identity_created: false,
            }) if armed == session_generation
        )
    }
}

/// Whether this device still needs verification: the account has an
/// identity and that identity has not cross-signed this device.
///
/// Waits for the SDK's sign-in E2EE tasks first (recovery state, backup
/// resume), so the answer reflects what they found.
pub async fn device_needs_verification(client: &Client) -> Result<bool, &'static str> {
    let encryption = client.encryption();
    encryption.wait_for_e2ee_initialization_tasks().await;
    let user_id = client
        .user_id()
        .ok_or("v-crypto.2-cross-signing-user-missing")?;
    if query_own_identity(&encryption, user_id).await?.is_none() {
        return Ok(false);
    }
    // Holding the whole private identity (after recovery, a reset, or secret
    // sharing) means this device signs itself; the signed device keys may
    // reach the local store only with the next own-user key query.
    if encryption
        .cross_signing_status()
        .await
        .is_some_and(|status| status.is_complete())
    {
        return Ok(false);
    }
    let device = encryption
        .get_own_device()
        .await
        .map_err(|_| "v-crypto.2-verify-prompt-device-unavailable")?;
    Ok(!device.is_some_and(|device| device.is_cross_signed_by_owner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_sign_in_that_found_an_existing_identity_may_show_the_prompt() {
        let found = DeviceVerificationPromptArm::after_sign_in(
            4,
            LoginCrossSigningOutcome::AlreadyConfigured,
        );
        assert!(DeviceVerificationPromptArm::may_show(Some(found), 4));
        assert!(!DeviceVerificationPromptArm::may_show(Some(found), 5));

        let created =
            DeviceVerificationPromptArm::after_sign_in(4, LoginCrossSigningOutcome::Created);
        assert!(!DeviceVerificationPromptArm::may_show(Some(created), 4));

        let dismissed = DeviceVerificationPromptArm::Dismissed {
            session_generation: 4,
        };
        assert!(!DeviceVerificationPromptArm::may_show(Some(dismissed), 4));
        assert!(!DeviceVerificationPromptArm::may_show(None, 4));
    }
}
