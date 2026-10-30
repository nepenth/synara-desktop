//! S5 identity reset for an account that has lost its recovery key and every
//! other device, over the SDK's `Recovery::reset_identity`.
//!
//! The SDK first deletes key backup and disables secret storage, then uploads
//! a new cross-signing identity. That upload usually needs the account
//! password or, on OAuth 2.0 servers, approval on the account page. The
//! pending `IdentityResetHandle` lives in the session's device owner, so
//! sign-out drops (and cancels) it. No password, UIA session or key leaves
//! Core; the approval URL is the only homeserver value returned.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use matrix_sdk::{
    encryption::{
        recovery::{IdentityResetHandle, RecoveryError},
        CrossSigningResetAuthType,
    },
    ruma::api::client::uiaa::{AuthData, Dummy},
    Client, HttpError,
};
use serde::{Deserialize, Serialize};

use super::live::{password_auth, supported_authentication, SupportedBootstrapAuthentication};

pub const IDENTITY_RESET_NOT_PENDING: &str = "v-crypto.2-identity-reset-not-pending";
pub const IDENTITY_RESET_PASSWORD_EMPTY: &str = "v-crypto.2-identity-reset-password-empty";
pub const IDENTITY_RESET_PASSWORD_REJECTED: &str = "v-crypto.2-identity-reset-password-rejected";
pub const IDENTITY_RESET_APPROVAL_PENDING: &str = "v-crypto.2-identity-reset-approval-pending";
pub const IDENTITY_RESET_NETWORK: &str = "v-crypto.2-identity-reset-network";
pub const IDENTITY_RESET_AUTH_UNSUPPORTED: &str = "v-crypto.2-identity-reset-auth-unsupported";
pub const IDENTITY_RESET_CANCELLED: &str = "v-crypto.2-identity-reset-cancelled";
pub const IDENTITY_RESET_FAILED: &str = "v-crypto.2-identity-reset-failed";
const IDENTITY_RESET_STATE_POISONED: &str = "v-crypto.2-identity-reset-state-poisoned";
const IDENTITY_RESET_USER_MISSING: &str = "v-crypto.2-cross-signing-user-missing";

/// How long one authentication step waits for the homeserver to accept the
/// upload. The SDK keeps polling for two minutes while the server answers
/// 401; the user approves before pressing Continue, so a short wait answers
/// quickly when they have not, and Continue can be pressed again.
pub const IDENTITY_RESET_AUTH_WAIT: Duration = Duration::from_secs(10);

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeIdentityResetStep {
    /// The new identity is published. Recovery setup comes next.
    Complete,
    /// The homeserver wants the account password.
    PasswordRequired,
    /// Approve the reset on the account page (`approval_url`), then continue.
    ApprovalRequired,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeIdentityResetState {
    pub session_generation: u64,
    pub step: NativeIdentityResetStep,
    /// Only for `ApprovalRequired`: the homeserver's account-page URL.
    pub approval_url: Option<String>,
}

enum PendingAuth {
    Password { auth_session: Option<String> },
    Approval { approval_url: String },
}

struct PendingReset {
    handle: Arc<IdentityResetHandle>,
    auth: PendingAuth,
}

/// The session's one pending identity reset, between Start and the password
/// or approval step.
pub struct IdentityResetFlow {
    session_generation: u64,
    wait: Duration,
    pending: Mutex<Option<PendingReset>>,
}

impl IdentityResetFlow {
    pub fn new(session_generation: u64) -> Self {
        Self::with_wait(session_generation, IDENTITY_RESET_AUTH_WAIT)
    }

    pub(crate) fn with_wait(session_generation: u64, wait: Duration) -> Self {
        Self {
            session_generation,
            wait,
            pending: Mutex::new(None),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending
            .lock()
            .map(|pending| pending.is_some())
            .unwrap_or(false)
    }

    /// Start a reset: the SDK deletes key backup, disables secret storage and
    /// tries to upload a new identity. Returns which step comes next. A reset
    /// left pending earlier is cancelled first.
    pub async fn start(&self, client: &Client) -> Result<NativeIdentityResetState, &'static str> {
        self.cancel(client).await;
        let result = self.begin(client).await;
        if result.is_err() {
            resync_published_identity(client).await;
        }
        result
    }

    async fn begin(&self, client: &Client) -> Result<NativeIdentityResetState, &'static str> {
        let handle = match client.encryption().recovery().reset_identity().await {
            Ok(Some(handle)) => handle,
            Ok(None) => return self.complete(client).await,
            Err(error) => return Err(map_reset_error(&error)),
        };
        let auth = match handle.auth_type() {
            CrossSigningResetAuthType::OAuth(info) => PendingAuth::Approval {
                approval_url: info.approval_url.to_string(),
            },
            CrossSigningResetAuthType::Uiaa(info) => match supported_authentication(info) {
                Some(SupportedBootstrapAuthentication::Dummy) => {
                    let mut dummy = Dummy::new();
                    dummy.session = info.session.clone();
                    return match tokio::time::timeout(
                        self.wait,
                        handle.reset(Some(AuthData::Dummy(dummy))),
                    )
                    .await
                    {
                        Ok(Ok(())) => self.complete(client).await,
                        Ok(Err(error)) => Err(map_reset_error(&error)),
                        Err(_) => Err(IDENTITY_RESET_FAILED),
                    };
                }
                Some(SupportedBootstrapAuthentication::Password) => PendingAuth::Password {
                    auth_session: info.session.clone(),
                },
                None => return Err(IDENTITY_RESET_AUTH_UNSUPPORTED),
            },
        };
        let state = self.state_for(&auth);
        *self
            .pending
            .lock()
            .map_err(|_| IDENTITY_RESET_STATE_POISONED)? = Some(PendingReset {
            handle: Arc::new(handle),
            auth,
        });
        Ok(state)
    }

    /// Answer the password stage. A rejected password keeps the reset
    /// pending, so the user can try again.
    pub async fn authenticate_password(
        &self,
        client: &Client,
        password: &str,
    ) -> Result<NativeIdentityResetState, &'static str> {
        if password.is_empty() {
            return Err(IDENTITY_RESET_PASSWORD_EMPTY);
        }
        let (handle, auth_session) = {
            let pending = self
                .pending
                .lock()
                .map_err(|_| IDENTITY_RESET_STATE_POISONED)?;
            match pending.as_ref() {
                Some(PendingReset {
                    handle,
                    auth: PendingAuth::Password { auth_session },
                }) => (Arc::clone(handle), auth_session.clone()),
                _ => return Err(IDENTITY_RESET_NOT_PENDING),
            }
        };
        let user_id = client.user_id().ok_or(IDENTITY_RESET_USER_MISSING)?;
        let mut auth = password_auth(user_id, password);
        auth.session = auth_session;
        match tokio::time::timeout(self.wait, handle.reset(Some(AuthData::Password(auth)))).await {
            Ok(Ok(())) => self.finish(client, &handle).await,
            Ok(Err(error)) => match uiaa_session(&error) {
                Some(next_session) => {
                    self.rearm_password_session(&handle, next_session)?;
                    Err(IDENTITY_RESET_PASSWORD_REJECTED)
                }
                None => Err(map_reset_error(&error)),
            },
            // The homeserver kept asking for authentication.
            Err(_) => Err(IDENTITY_RESET_PASSWORD_REJECTED),
        }
    }

    /// Continue after the user approved the reset on the account page. Not
    /// approved yet keeps the reset pending.
    pub async fn continue_after_approval(
        &self,
        client: &Client,
    ) -> Result<NativeIdentityResetState, &'static str> {
        let handle = {
            let pending = self
                .pending
                .lock()
                .map_err(|_| IDENTITY_RESET_STATE_POISONED)?;
            match pending.as_ref() {
                Some(PendingReset {
                    handle,
                    auth: PendingAuth::Approval { .. },
                }) => Arc::clone(handle),
                _ => return Err(IDENTITY_RESET_NOT_PENDING),
            }
        };
        match tokio::time::timeout(self.wait, handle.reset(None)).await {
            Ok(Ok(())) => self.finish(client, &handle).await,
            Ok(Err(RecoveryError::Sdk(matrix_sdk::Error::Timeout))) | Err(_) => {
                Err(IDENTITY_RESET_APPROVAL_PENDING)
            }
            Ok(Err(error)) => Err(map_reset_error(&error)),
        }
    }

    /// Drop the pending reset. A Continue that is still polling stops, and
    /// the unpublished keys the reset saved on this device are dropped.
    pub async fn cancel(&self, client: &Client) {
        if self.abandon().await {
            resync_published_identity(client).await;
        }
    }

    /// Drop the pending reset without touching the homeserver (sign-out).
    /// Returns whether one was pending.
    pub async fn abandon(&self) -> bool {
        let pending = self
            .pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.take());
        match pending {
            Some(pending) => {
                pending.handle.cancel().await;
                true
            }
            None => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_handle(&self) -> Option<std::sync::Weak<IdentityResetHandle>> {
        self.pending
            .lock()
            .ok()?
            .as_ref()
            .map(|pending| Arc::downgrade(&pending.handle))
    }

    fn rearm_password_session(
        &self,
        handle: &Arc<IdentityResetHandle>,
        next_session: Option<String>,
    ) -> Result<(), &'static str> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| IDENTITY_RESET_STATE_POISONED)?;
        if let Some(PendingReset {
            handle: current,
            auth: PendingAuth::Password { auth_session },
        }) = pending.as_mut()
        {
            if Arc::ptr_eq(current, handle) && next_session.is_some() {
                *auth_session = next_session;
            }
        }
        Ok(())
    }

    /// The upload went through, unless a cancel stopped it: the SDK then
    /// returns success without uploading.
    async fn finish(
        &self,
        client: &Client,
        handle: &Arc<IdentityResetHandle>,
    ) -> Result<NativeIdentityResetState, &'static str> {
        let was_pending = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| IDENTITY_RESET_STATE_POISONED)?;
            match pending.as_ref() {
                Some(current) if Arc::ptr_eq(&current.handle, handle) => {
                    *pending = None;
                    true
                }
                _ => false,
            }
        };
        if !was_pending {
            return Err(IDENTITY_RESET_CANCELLED);
        }
        self.complete(client).await
    }

    async fn complete(&self, client: &Client) -> Result<NativeIdentityResetState, &'static str> {
        let published = client
            .encryption()
            .cross_signing_status()
            .await
            .is_some_and(|status| status.is_complete());
        if !published {
            return Err(IDENTITY_RESET_FAILED);
        }
        // Drafts sealed under the old identity's key must not follow it.
        crate::app::drafts::on_identity_reset(client).await;
        Ok(NativeIdentityResetState {
            session_generation: self.session_generation,
            step: NativeIdentityResetStep::Complete,
            approval_url: None,
        })
    }

    fn state_for(&self, auth: &PendingAuth) -> NativeIdentityResetState {
        let (step, approval_url) = match auth {
            PendingAuth::Password { .. } => (NativeIdentityResetStep::PasswordRequired, None),
            PendingAuth::Approval { approval_url } => (
                NativeIdentityResetStep::ApprovalRequired,
                Some(approval_url.clone()),
            ),
        };
        NativeIdentityResetState {
            session_generation: self.session_generation,
            step,
            approval_url,
        }
    }
}

impl Drop for IdentityResetFlow {
    /// Sign-out drops the device owner: stop a Continue still polling.
    fn drop(&mut self) {
        let pending = self
            .pending
            .get_mut()
            .ok()
            .and_then(|pending| pending.take());
        if let (Some(pending), Ok(runtime)) = (pending, tokio::runtime::Handle::try_current()) {
            runtime.spawn(async move { pending.handle.cancel().await });
        }
    }
}

/// The SDK saves the new private identity on this device before it asks
/// for authentication, replacing the keys of the published identity. After a
/// cancel or a failed start, an own-user key query lets the crypto machine
/// drop private keys that do not match the published identity, so nothing
/// (recovery setup above all) stores or signs with the unpublished ones.
async fn resync_published_identity(client: &Client) {
    if let Some(user_id) = client.user_id() {
        let _ = tokio::time::timeout(
            Duration::from_secs(8),
            client.encryption().request_user_identity(user_id),
        )
        .await;
    }
}

/// The UIA challenge a refused password brought back, with its session.
fn uiaa_session(error: &RecoveryError) -> Option<Option<String>> {
    match error {
        RecoveryError::Sdk(error) => error.as_uiaa_response().map(|info| info.session.clone()),
        _ => None,
    }
}

fn map_reset_error(error: &RecoveryError) -> &'static str {
    match error {
        RecoveryError::Sdk(matrix_sdk::Error::Http(http))
            if matches!(**http, HttpError::Reqwest(_)) =>
        {
            IDENTITY_RESET_NETWORK
        }
        _ => IDENTITY_RESET_FAILED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_state_carries_no_secret_or_uia_session() {
        let state = NativeIdentityResetState {
            session_generation: 3,
            step: NativeIdentityResetStep::ApprovalRequired,
            approval_url: Some("https://account.example.org/reset".to_owned()),
        };
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(
            json,
            r#"{"sessionGeneration":3,"step":"approval_required","approvalUrl":"https://account.example.org/reset"}"#
        );
        for forbidden in ["password", "session\"", "auth", "key"] {
            assert!(!json.to_ascii_lowercase().contains(forbidden));
        }
    }
}
