//! Live V-CRYPTO.2 cross-signing status, setup, password UIA and the
//! first-login bootstrap.

use std::time::Duration;

use matrix_sdk::{
    encryption::{identities::UserIdentity, CrossSigningStatus, Encryption},
    ruma::{
        api::client::{
            keys::get_keys,
            uiaa::{
                AuthData, AuthType, Dummy, MatrixUserIdentifier, Password, UiaaInfo, UserIdentifier,
            },
        },
        UserId,
    },
    Client,
};

use super::{
    project_cross_signing_status, NativeCrossSigningBootstrap, NativeCrossSigningPrivateFlags,
    NativeCrossSigningPrivateIdentity, NativeCrossSigningSetupOutcome,
    NativeCrossSigningSetupResult, NativeCrossSigningStatus,
};
use crate::app::devices::OwnKeysQueryGate;
use crate::platform::{
    PlatformCrossSigningOwnIdentity, PlatformCrossSigningPrivateState, PlatformCrossSigningStatus,
    PlatformCrossSigningStatusError, PlatformCryptoCrossSigningState, PlatformCryptoStatus,
    PlatformCryptoStatusError,
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

/// Local identity first; bound the homeserver `/keys/query` so iOS Settings
/// cannot stall on a spinner the way desktop Devices did.
pub async fn query_own_identity(
    encryption: &Encryption,
    user_id: &UserId,
) -> Result<Option<UserIdentity>, &'static str> {
    match encryption
        .get_user_identity(user_id)
        .await
        .map_err(|_| "v-crypto.2-cross-signing-identity-query-failed")?
    {
        Some(identity) => Ok(Some(identity)),
        None => match tokio::time::timeout(
            Duration::from_secs(8),
            encryption.request_user_identity(user_id),
        )
        .await
        {
            Ok(Ok(identity)) => Ok(identity),
            Ok(Err(_)) => Err("v-crypto.2-cross-signing-identity-query-failed"),
            Err(_) => Ok(None),
        },
    }
}

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeCrossSigningStatus, &'static str> {
    let encryption = client.encryption();
    let private_status = encryption.cross_signing_status().await;
    let user_id = client
        .user_id()
        .ok_or("v-crypto.2-cross-signing-user-missing")?;
    let own_identity = query_own_identity(&encryption, user_id).await?;
    let published = own_identity.is_some()
        || private_status
            .as_ref()
            .is_some_and(|status| status.is_complete());
    Ok(project_status(
        session_generation,
        private_status.as_ref(),
        published,
        own_identity
            .as_ref()
            .is_some_and(|identity| identity.is_verified()),
    ))
}

/// The closed crypto projection Core serves as `crypto_status`. Local only:
/// the SDK's private cross-signing status needs no homeserver request.
pub async fn crypto_status_projection(
    client: &Client,
    session_generation: u64,
) -> Result<PlatformCryptoStatus, PlatformCryptoStatusError> {
    let (encryption_enabled, state) = match client.encryption().cross_signing_status().await {
        None => (false, PlatformCryptoCrossSigningState::Unavailable),
        Some(status) if status.is_complete() => (true, PlatformCryptoCrossSigningState::Ready),
        Some(status) if private_flags(&status).has_any() => {
            (true, PlatformCryptoCrossSigningState::Partial)
        }
        Some(_) => (true, PlatformCryptoCrossSigningState::NotSetUp),
    };
    PlatformCryptoStatus::new(session_generation, encryption_enabled, state)
}

/// The closed projection Core serves as `cross_signing_status`.
///
/// Same rules as the desktop shell's projection: the local identity first,
/// then at most one bounded own-user `/keys/query` per session generation
/// through the device owner's gate. Once that query ran, the SDK keeps the
/// own identity current from sync, so a missing one is missing.
pub async fn cross_signing_status_projection(
    client: &Client,
    session_generation: u64,
    own_keys_query: &OwnKeysQueryGate,
) -> Result<PlatformCrossSigningStatus, PlatformCrossSigningStatusError> {
    let encryption = client.encryption();
    let private_status = encryption.cross_signing_status().await;
    let user_id = client
        .user_id()
        .ok_or(PlatformCrossSigningStatusError::UserMissing)?;
    let local_identity = encryption
        .get_user_identity(user_id)
        .await
        .map_err(|_| PlatformCrossSigningStatusError::IdentityQueryFailed)?;
    let own_identity = match local_identity {
        Some(identity) => Some(identity),
        None if !own_keys_query.try_begin() => None,
        None => {
            let fetched = tokio::time::timeout(
                Duration::from_secs(8),
                encryption.request_user_identity(user_id),
            )
            .await;
            own_keys_query.finish(matches!(fetched, Ok(Ok(_))));
            match fetched {
                Ok(Ok(identity)) => identity,
                Ok(Err(_)) => return Err(PlatformCrossSigningStatusError::IdentityQueryFailed),
                Err(_) => None,
            }
        }
    };
    let private_state = match private_status.as_ref() {
        None => PlatformCrossSigningPrivateState::Unavailable,
        Some(status) if status.is_complete() => PlatformCrossSigningPrivateState::Complete,
        Some(status) if private_flags(status).has_any() => {
            PlatformCrossSigningPrivateState::Partial
        }
        Some(_) => PlatformCrossSigningPrivateState::Missing,
    };
    let own_identity = match own_identity.as_ref() {
        Some(identity) if identity.is_verified() => PlatformCrossSigningOwnIdentity::Verified,
        Some(_) => PlatformCrossSigningOwnIdentity::Unverified,
        // Local private keys exist but the identity query did not return.
        // Offer verification instead of reporting no identity.
        None if private_state == PlatformCrossSigningPrivateState::Complete => {
            PlatformCrossSigningOwnIdentity::Unverified
        }
        None => PlatformCrossSigningOwnIdentity::Missing,
    };
    PlatformCrossSigningStatus::new(session_generation, private_state, own_identity)
}

fn private_flags(status: &CrossSigningStatus) -> NativeCrossSigningPrivateFlags {
    NativeCrossSigningPrivateFlags {
        has_master: status.has_master,
        has_self_signing: status.has_self_signing,
        has_user_signing: status.has_user_signing,
    }
}

/// Some, but not all, private keys on this device and no published identity.
///
/// The SDK would upload the partial identity it holds rather than create a
/// new one, so every bootstrap path refuses this state. The missing keys come
/// back through recovery, or verification with the device that has them.
fn refuse_partial_private_identity(status: &NativeCrossSigningStatus) -> Result<(), &'static str> {
    if status.private_identity == NativeCrossSigningPrivateIdentity::Partial {
        return Err("v-crypto.2-cross-signing-partial-private-identity");
    }
    Ok(())
}

/// Whether the homeserver holds a master key for this account.
///
/// Asked directly rather than through the local store: an upload attempt
/// saves the new identity locally *before* sending it, so after a refused or
/// abandoned UIA step the store holds an identity the server never got. The
/// response is only inspected, never fed to the crypto machine.
async fn homeserver_has_own_identity(
    client: &Client,
    user_id: &UserId,
) -> Result<bool, &'static str> {
    let mut request = get_keys::v3::Request::new();
    request.device_keys.insert(user_id.to_owned(), Vec::new());
    let response = client
        .send(request)
        .await
        .map_err(|_| "v-crypto.2-cross-signing-identity-query-failed")?;
    Ok(response.master_keys.contains_key(user_id))
}

/// Send this device's identity. Callers first confirm the homeserver has
/// none (`homeserver_has_own_identity`), so another device's identity is never
/// replaced. MSC3967 servers accept this first upload without UIA; otherwise
/// the UIA challenge comes back.
///
/// `bootstrap_cross_signing` reuses an identity an earlier attempt saved
/// locally and creates one otherwise, so every retry, including the UIA
/// retries that follow, re-sends the same keys, never new ones.
async fn upload_identity(client: &Client) -> Result<Option<UiaaInfo>, &'static str> {
    match client.encryption().bootstrap_cross_signing(None).await {
        Ok(()) => Ok(None),
        Err(error) => error
            .as_uiaa_response()
            .cloned()
            .map(Some)
            .ok_or("v-crypto.2-cross-signing-bootstrap-failed"),
    }
}

pub async fn setup(
    client: &Client,
    session_generation: u64,
) -> Result<(NativeCrossSigningSetupResult, Option<String>), &'static str> {
    let before = status(client, session_generation).await?;
    let user_id = client
        .user_id()
        .ok_or("v-crypto.2-cross-signing-user-missing")?;
    if homeserver_has_own_identity(client, user_id).await? {
        return Ok((
            NativeCrossSigningSetupResult {
                outcome: NativeCrossSigningSetupOutcome::AlreadyConfigured,
                status: before,
            },
            None,
        ));
    }
    refuse_partial_private_identity(&before)?;

    match upload_identity(client).await? {
        None => complete(client, session_generation)
            .await
            .map(|result| (result, None)),
        Some(info) => match supported_authentication(&info) {
            Some(SupportedBootstrapAuthentication::Dummy) => {
                let mut dummy = Dummy::new();
                dummy.session = info.session.clone();
                client
                    .encryption()
                    .bootstrap_cross_signing(Some(AuthData::Dummy(dummy)))
                    .await
                    .map_err(|_| "v-crypto.2-cross-signing-dummy-auth-failed")?;
                complete(client, session_generation)
                    .await
                    .map(|result| (result, None))
            }
            Some(SupportedBootstrapAuthentication::Password) => {
                let auth_session = info
                    .session
                    .clone()
                    .ok_or("v-crypto.2-cross-signing-auth-session-missing")?;
                let status = status(client, session_generation).await?;
                Ok((
                    NativeCrossSigningSetupResult {
                        outcome: NativeCrossSigningSetupOutcome::AuthenticationRequired,
                        status,
                    },
                    Some(auth_session),
                ))
            }
            None => Err("v-crypto.2-cross-signing-auth-unsupported"),
        },
    }
}

/// Why a password retry did not finish setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossSigningPasswordError {
    /// The homeserver refused the password. Retry with this UIA session.
    Rejected { auth_session: Option<String> },
    /// Any other failure; the static diagnostic only.
    Failed(&'static str),
}

/// Finish a setup that `setup` left at `AuthenticationRequired` by answering
/// the password stage of its UIA session.
///
/// Desktop and iOS share this. `setup` already established, after the
/// own-user `/keys/query`, that the account had no identity; this re-sends
/// the identity that attempt saved locally, so a retry never creates a second
/// one. A partial private identity is refused here too.
pub async fn authenticate_password(
    client: &Client,
    session_generation: u64,
    auth_session: String,
    password: &str,
) -> Result<NativeCrossSigningSetupResult, CrossSigningPasswordError> {
    if password.is_empty() {
        return Err(CrossSigningPasswordError::Failed(
            "v-crypto.2-cross-signing-password-empty",
        ));
    }
    // Read the SDK's private flags directly: `status()` counts the identity
    // the first attempt saved locally as published.
    match client.encryption().cross_signing_status().await {
        None => {
            return Err(CrossSigningPasswordError::Failed(
                "v-crypto.2-cross-signing-crypto-unavailable",
            ))
        }
        Some(private) if !private.is_complete() && private_flags(&private).has_any() => {
            return Err(CrossSigningPasswordError::Failed(
                "v-crypto.2-cross-signing-partial-private-identity",
            ))
        }
        Some(_) => {}
    }
    let user_id = client.user_id().ok_or(CrossSigningPasswordError::Failed(
        "v-crypto.2-cross-signing-user-missing",
    ))?;
    let mut auth = password_auth(user_id, password);
    auth.session = Some(auth_session);
    if let Err(error) = client
        .encryption()
        .bootstrap_cross_signing(Some(AuthData::Password(auth)))
        .await
    {
        return Err(match error.as_uiaa_response() {
            Some(info) => CrossSigningPasswordError::Rejected {
                auth_session: info.session.clone(),
            },
            None => CrossSigningPasswordError::Failed("v-crypto.2-cross-signing-auth-failed"),
        });
    }
    complete(client, session_generation)
        .await
        .map_err(CrossSigningPasswordError::Failed)
}

pub(super) fn password_auth(user_id: &UserId, password: &str) -> Password {
    Password::new(
        UserIdentifier::Matrix(MatrixUserIdentifier::new(user_id.to_string())),
        password.to_owned(),
    )
}

/// What the sign-in bootstrap did. Sign-in never fails because of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginCrossSigningOutcome {
    /// This device created and published the account's identity.
    Created,
    /// The account already had an identity; this device left it alone.
    AlreadyConfigured,
    /// Nothing was uploaded. The static diagnostic says why; the user can
    /// still set up from Security settings.
    Skipped(&'static str),
}

/// Bootstrap cross-signing on the device a password sign-in just created, so
/// an account's first device is cross-signed (MSC4153) without a settings
/// visit. It is the same guarded upload as `setup`: only when the homeserver
/// reports no identity for the account, and never over a partial private
/// identity. MSC3967 servers accept the first upload without UIA; otherwise
/// the sign-in password answers the password stage. Other UIA flows (SSO,
/// OAuth approval) are left for the user.
///
/// It then waits for the SDK's sign-in E2EE tasks (backup resume, recovery
/// state), so the first status read already reflects the identity.
pub async fn bootstrap_after_login(client: &Client, password: &str) -> LoginCrossSigningOutcome {
    let outcome = bootstrap_after_login_inner(client, password).await;
    client
        .encryption()
        .wait_for_e2ee_initialization_tasks()
        .await;
    outcome
}

async fn bootstrap_after_login_inner(client: &Client, password: &str) -> LoginCrossSigningOutcome {
    let encryption = client.encryption();
    let Some(before) = encryption.cross_signing_status().await else {
        return LoginCrossSigningOutcome::Skipped("v-crypto.2-login-bootstrap-crypto-unavailable");
    };
    let Some(user_id) = client.user_id() else {
        return LoginCrossSigningOutcome::Skipped("v-crypto.2-cross-signing-user-missing");
    };
    match homeserver_has_own_identity(client, user_id).await {
        Ok(true) => return LoginCrossSigningOutcome::AlreadyConfigured,
        Ok(false) => {}
        Err(diagnostic) => return LoginCrossSigningOutcome::Skipped(diagnostic),
    }
    if !before.is_complete() && private_flags(&before).has_any() {
        return LoginCrossSigningOutcome::Skipped(
            "v-crypto.2-cross-signing-partial-private-identity",
        );
    }
    let challenge = match upload_identity(client).await {
        Ok(challenge) => challenge,
        Err(_) => return LoginCrossSigningOutcome::Skipped("v-crypto.2-login-bootstrap-failed"),
    };
    if let Some(info) = challenge {
        let auth = match supported_authentication(&info) {
            Some(SupportedBootstrapAuthentication::Dummy) => {
                let mut dummy = Dummy::new();
                dummy.session = info.session.clone();
                AuthData::Dummy(dummy)
            }
            Some(SupportedBootstrapAuthentication::Password) if !password.is_empty() => {
                let mut auth = password_auth(user_id, password);
                auth.session = info.session.clone();
                AuthData::Password(auth)
            }
            _ => {
                return LoginCrossSigningOutcome::Skipped(
                    "v-crypto.2-cross-signing-auth-unsupported",
                )
            }
        };
        // Re-sends the identity the first attempt saved locally.
        if encryption
            .bootstrap_cross_signing(Some(auth))
            .await
            .is_err()
        {
            return LoginCrossSigningOutcome::Skipped("v-crypto.2-cross-signing-auth-failed");
        }
    }
    LoginCrossSigningOutcome::Created
}

pub async fn complete(
    client: &Client,
    session_generation: u64,
) -> Result<NativeCrossSigningSetupResult, &'static str> {
    let status = status(client, session_generation).await?;
    if status.bootstrap == NativeCrossSigningBootstrap::Needed {
        return Err("v-crypto.2-cross-signing-bootstrap-incomplete");
    }
    Ok(NativeCrossSigningSetupResult {
        outcome: NativeCrossSigningSetupOutcome::Complete,
        status,
    })
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

    #[test]
    fn own_identity_lookup_prefers_local_store_and_bounds_keys_query() {
        let source = include_str!("live.rs");
        let helper = source
            .split("pub async fn query_own_identity")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn status").next())
            .expect("query_own_identity helper");
        assert!(helper.contains("get_user_identity(user_id)"));
        assert!(helper.contains("Duration::from_secs(8)"));
        assert!(helper.contains("request_user_identity(user_id)"));
        let local_idx = helper
            .find("get_user_identity(user_id)")
            .expect("local first");
        let remote_idx = helper
            .find("request_user_identity(user_id)")
            .expect("bounded remote");
        assert!(
            local_idx < remote_idx,
            "local identity must be read before /keys/query"
        );
    }
}
