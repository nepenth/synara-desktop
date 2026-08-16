//! Live V-CRYPTO.2 cross-signing status and setup start.

use matrix_sdk::{
    encryption::CrossSigningStatus,
    ruma::api::client::uiaa::{AuthData, AuthType, Dummy, UiaaInfo},
    Client,
};

use super::{
    project_cross_signing_status, NativeCrossSigningBootstrap, NativeCrossSigningPrivateFlags,
    NativeCrossSigningSetupOutcome, NativeCrossSigningSetupResult, NativeCrossSigningStatus,
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

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeCrossSigningStatus, &'static str> {
    let encryption = client.encryption();
    let private_status = encryption.cross_signing_status().await;
    let user_id = client
        .user_id()
        .ok_or("v-crypto.2-cross-signing-user-missing")?;
    let own_identity = encryption
        .request_user_identity(user_id)
        .await
        .map_err(|_| "v-crypto.2-cross-signing-identity-query-failed")?;
    Ok(project_status(
        session_generation,
        private_status.as_ref(),
        own_identity.is_some(),
        own_identity
            .as_ref()
            .is_some_and(|identity| identity.is_verified()),
    ))
}

pub async fn setup(
    client: &Client,
    session_generation: u64,
) -> Result<(NativeCrossSigningSetupResult, Option<String>), &'static str> {
    let before = status(client, session_generation).await?;
    if before.bootstrap != NativeCrossSigningBootstrap::Needed {
        return Ok((
            NativeCrossSigningSetupResult {
                outcome: NativeCrossSigningSetupOutcome::AlreadyConfigured,
                status: before,
            },
            None,
        ));
    }

    match client
        .encryption()
        .bootstrap_cross_signing_if_needed(None)
        .await
    {
        Ok(()) => complete(client, session_generation)
            .await
            .map(|result| (result, None)),
        Err(error) => {
            let info = error
                .as_uiaa_response()
                .ok_or("v-crypto.2-cross-signing-bootstrap-failed")?;
            match supported_authentication(info) {
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
            }
        }
    }
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
}
