//! Core adapters for identity reset and the post-sign-in "Verify this
//! device" prompt (S5). The account password is a method argument only.

use super::*;
use crate::app::cross_signing::{
    DeviceVerificationPromptArm, LoginCrossSigningOutcome, NativeDeviceVerificationPrompt,
    NativeIdentityResetState, IDENTITY_RESET_AUTH_UNSUPPORTED, IDENTITY_RESET_NETWORK,
    IDENTITY_RESET_PASSWORD_EMPTY, IDENTITY_RESET_PASSWORD_REJECTED,
};

const IDENTITY_RESET_NO_SESSION: &str = "p2-identity-reset-no-session";
const VERIFY_PROMPT_NO_SESSION: &str = "p2-verify-prompt-no-session";

impl Core {
    /// Start an identity reset for this session's account. Returns the next
    /// step: done, the account password, or approval on the account page.
    pub async fn identity_reset_start(&self) -> Result<NativeIdentityResetState, MatrixIpcError> {
        self.identity_reset_owner()?
            .identity_reset_start()
            .await
            .map_err(identity_reset_error)
    }

    /// Answer the password stage of the pending reset.
    pub async fn identity_reset_password(
        &self,
        password: &str,
    ) -> Result<NativeIdentityResetState, MatrixIpcError> {
        if password.is_empty() || password.len() > 100_000 {
            return Err(identity_reset_error(IDENTITY_RESET_PASSWORD_EMPTY));
        }
        self.identity_reset_owner()?
            .identity_reset_password(password)
            .await
            .map_err(identity_reset_error)
    }

    /// Continue the pending reset after the user approved it in the browser.
    pub async fn identity_reset_continue(
        &self,
    ) -> Result<NativeIdentityResetState, MatrixIpcError> {
        self.identity_reset_owner()?
            .identity_reset_continue()
            .await
            .map_err(identity_reset_error)
    }

    /// Drop the pending reset. Nothing to cancel without a session.
    pub async fn identity_reset_cancel(&self) -> Result<(), MatrixIpcError> {
        if let Some(owner) = self.state.device_owner()? {
            owner.identity_reset_cancel().await;
        }
        Ok(())
    }

    /// Arm the one-time prompt for the session a password sign-in just
    /// opened, with what the sign-in bootstrap did. Restores never call this.
    pub fn arm_device_verification_prompt(
        &self,
        outcome: LoginCrossSigningOutcome,
    ) -> Result<(), MatrixIpcError> {
        let session_generation = self.prompt_session_generation()?;
        *self.verification_prompt()? = Some(DeviceVerificationPromptArm::after_sign_in(
            session_generation,
            outcome,
        ));
        Ok(())
    }

    /// Whether to show "Verify this device" now: armed by this session's
    /// sign-in, not dismissed, the account already had an identity, and that
    /// identity has not verified this device.
    pub async fn device_verification_prompt(
        &self,
    ) -> Result<NativeDeviceVerificationPrompt, MatrixIpcError> {
        let session_generation = self.prompt_session_generation()?;
        let arm = *self.verification_prompt()?;
        let mut show = DeviceVerificationPromptArm::may_show(arm, session_generation);
        if show {
            let owner = self.state.device_owner()?.ok_or_else(|| {
                MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                    .with_diagnostic(VERIFY_PROMPT_NO_SESSION)
            })?;
            show = owner
                .device_needs_verification()
                .await
                .map_err(|diagnostic| {
                    MatrixIpcError::new(MatrixIpcErrorCategory::CryptoFailure)
                        .with_diagnostic(diagnostic)
                })?;
        }
        Ok(NativeDeviceVerificationPrompt {
            session_generation,
            show,
        })
    }

    /// "Later": the prompt stays hidden for this session.
    pub fn dismiss_device_verification_prompt(
        &self,
    ) -> Result<NativeDeviceVerificationPrompt, MatrixIpcError> {
        let session_generation = self.prompt_session_generation()?;
        *self.verification_prompt()? =
            Some(DeviceVerificationPromptArm::Dismissed { session_generation });
        Ok(NativeDeviceVerificationPrompt {
            session_generation,
            show: false,
        })
    }

    fn identity_reset_owner(&self) -> Result<Arc<NativeDeviceOwner>, MatrixIpcError> {
        self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic(IDENTITY_RESET_NO_SESSION)
        })
    }

    fn prompt_session_generation(&self) -> Result<u64, MatrixIpcError> {
        self.state
            .session_snapshot()?
            .map(|session| session.session_generation)
            .ok_or_else(|| {
                MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                    .with_diagnostic(VERIFY_PROMPT_NO_SESSION)
            })
    }

    fn verification_prompt(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<DeviceVerificationPromptArm>>, MatrixIpcError>
    {
        self.state
            .verification_prompt
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }
}

fn identity_reset_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        IDENTITY_RESET_PASSWORD_REJECTED
        | IDENTITY_RESET_AUTH_UNSUPPORTED
        | "v-crypto.2-cross-signing-user-missing" => MatrixIpcErrorCategory::Forbidden,
        IDENTITY_RESET_NETWORK => MatrixIpcErrorCategory::Connectivity,
        _ => MatrixIpcErrorCategory::CryptoFailure,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}
