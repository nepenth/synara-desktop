//! V-AUTH.4a — native password-reset (forgot password) via Matrix CS API.
//!
//! Owns the unauthenticated desktop happy path through the managed
//! `matrix_sdk` client + Ruma request types (no raw REST strings):
//! 1. email token request for password change (`request_password_change_token_via_email`)
//! 2. password change with UIAA email identity (`change_password`)
//! 3. Optional auto-completion of the `m.login.password` stage when the
//!    homeserver still requires it after email verification
//!
//! Secrets (new password, client secret) are never stored on the coordinator
//! and never appear in diagnostic ids or Display text. No dual-backend.

use matrix_sdk::ruma::{
    api::client::{
        account::{change_password, request_password_change_token_via_email},
        uiaa::{AuthData, AuthType, Password as UiaaPassword, UiaaInfo, UserIdentifier},
    },
    assign,
    thirdparty::Medium,
    ClientSecret, OwnedClientSecret, OwnedSessionId, UInt,
};
use matrix_sdk::{Client, Error as SdkError, HttpError};
use serde::Serialize;
use serde_json::{json, Map};

use super::error::AuthError;
use super::login::map_login_sdk_error;

/// Privacy-safe sid + optional submit URL from a password-reset email token request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordEmailTokenResult {
    /// Opaque 3PID session id (not an access token).
    pub sid: String,
    /// Optional identity-server submit URL; product may ignore when verification is out-of-band.
    pub submit_url: Option<String>,
}

/// Outcome of submitting password-reset UIAA stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PasswordResetOutcome {
    /// Password was changed successfully.
    Complete,
    /// Email stage still pending (user has not verified the link) or stage rejected.
    #[serde(rename_all = "camelCase")]
    EmailNotVerified {
        /// Opaque UIAA session string when the homeserver provided one.
        session: Option<String>,
        /// Stable Matrix errcode when present (e.g. `M_UNAUTHORIZED`); never a secret.
        error_code: Option<String>,
        /// Bounded static product message (never the raw homeserver body).
        error_message: Option<&'static str>,
    },
}

/// Request a password-reset email token from the homeserver.
///
/// `client` must be an unauthenticated P2.3 client pointed at the target homeserver.
pub async fn request_password_email_token(
    client: &Client,
    email: &str,
    client_secret: &str,
    send_attempt: u32,
) -> Result<PasswordEmailTokenResult, AuthError> {
    let email = validate_email(email)?;
    let secret = parse_client_secret(client_secret)?;
    let attempt = UInt::from(send_attempt);

    let request = request_password_change_token_via_email::v3::Request::new(secret, email, attempt);

    let response = client
        .send(request)
        .await
        .map_err(map_password_reset_http_error)?;

    Ok(PasswordEmailTokenResult {
        sid: response.sid.to_string(),
        submit_url: response.submit_url,
    })
}

/// Complete password reset: email identity stage, then password stage when required.
///
/// Matches the retained desktop product flow (logout other devices = false).
pub async fn complete_password_reset(
    client: &Client,
    email: &str,
    new_password: &str,
    client_secret: &str,
    sid: &str,
) -> Result<PasswordResetOutcome, AuthError> {
    let email = validate_email(email)?;
    validate_password_present(new_password)?;
    let secret = parse_client_secret(client_secret)?;
    let sid = parse_session_id(sid)?;

    let email_auth = email_identity_auth(sid.as_str(), secret.as_str(), None)?;

    match change_password(client, new_password, Some(email_auth)).await {
        Ok(()) => Ok(PasswordResetOutcome::Complete),
        Err(err) => {
            let Some(info) = err.as_uiaa_response() else {
                return Err(map_password_reset_http_error(err));
            };
            handle_uiaa_after_email_stage(client, &email, new_password, info).await
        }
    }
}

async fn handle_uiaa_after_email_stage(
    client: &Client,
    email: &str,
    new_password: &str,
    info: &UiaaInfo,
) -> Result<PasswordResetOutcome, AuthError> {
    if let Some(outcome) = email_not_verified_outcome(info) {
        return Ok(outcome);
    }

    if password_stage_required(info) {
        let session = info.session.clone();
        let mut password = UiaaPassword::new(
            UserIdentifier::third_party_id(Medium::Email, email.to_owned()),
            new_password.to_owned(),
        );
        password.session = session;
        let password_auth = AuthData::Password(password);

        return match change_password(client, new_password, Some(password_auth)).await {
            Ok(()) => Ok(PasswordResetOutcome::Complete),
            Err(err) => {
                let Some(info2) = err.as_uiaa_response() else {
                    return Err(map_password_reset_http_error(err));
                };
                if let Some(outcome) = email_not_verified_outcome(info2) {
                    return Ok(outcome);
                }
                // Remaining stages (recaptcha, terms, …) are out of V-AUTH.4a scope.
                if has_unsupported_remaining_stage(info2) {
                    return Err(AuthError::UnsupportedCapability {
                        diagnostic_id: "v-auth.4-password-reset-unsupported-uia-stage",
                    });
                }
                Ok(PasswordResetOutcome::EmailNotVerified {
                    session: info2.session.clone(),
                    error_code: auth_error_code(info2),
                    error_message: Some("Password reset authentication is incomplete."),
                })
            }
        };
    }

    if has_unsupported_remaining_stage(info) {
        return Err(AuthError::UnsupportedCapability {
            diagnostic_id: "v-auth.4-password-reset-unsupported-uia-stage",
        });
    }

    // Homeserver returned UIAA without a structured auth error — treat as
    // "continue after verifying email" for the product overlay.
    Ok(PasswordResetOutcome::EmailNotVerified {
        session: info.session.clone(),
        error_code: auth_error_code(info),
        error_message: None,
    })
}

async fn change_password(
    client: &Client,
    new_password: &str,
    auth: Option<AuthData>,
) -> Result<(), HttpError> {
    // Match matrix-js-sdk setPassword(..., logoutDevices=false).
    let request = assign!(change_password::v3::Request::new(new_password.to_owned()), {
        auth: auth,
        logout_devices: false,
    });
    client.send(request).await.map(|_| ())
}

/// Build `m.login.email.identity` AuthData without constructing non_exhaustive structs.
fn email_identity_auth(
    sid: &str,
    client_secret: &str,
    session: Option<String>,
) -> Result<AuthData, AuthError> {
    let mut data = Map::new();
    data.insert(
        "threepid_creds".into(),
        json!({
            "sid": sid,
            "client_secret": client_secret,
        }),
    );
    AuthData::new("m.login.email.identity", session, data).map_err(|_| AuthError::InvalidInput {
        diagnostic_id: "v-auth.4-email-auth-encode",
        reason: "failed to encode email identity auth",
    })
}

fn email_not_verified_outcome(info: &UiaaInfo) -> Option<PasswordResetOutcome> {
    let code = auth_error_code(info)?;
    // Common homeserver codes when the user has not clicked the email link yet,
    // or threepid credentials are wrong/stale.
    let message = match code.as_str() {
        "M_UNAUTHORIZED" | "M_FORBIDDEN" => {
            Some("Email has not been verified yet, or the verification expired.")
        }
        "M_THREEPID_AUTH_FAILED" | "M_THREEPID_IN_USE" | "M_THREEPID_DENIED" => {
            Some("Email verification failed. Request a new verification email.")
        }
        "M_NOT_FOUND" => Some("No account is associated with this email on the homeserver."),
        _ => Some("Password reset authentication was rejected."),
    };
    Some(PasswordResetOutcome::EmailNotVerified {
        session: info.session.clone(),
        error_code: Some(code),
        error_message: message,
    })
}

fn password_stage_required(info: &UiaaInfo) -> bool {
    if info.completed.contains(&AuthType::Password) {
        return false;
    }
    info.flows.iter().any(|flow| {
        flow.stages
            .iter()
            .any(|stage| matches!(stage, AuthType::Password))
    })
}

fn is_supported_password_reset_stage(stage: &AuthType) -> bool {
    matches!(
        stage,
        AuthType::Password | AuthType::EmailIdentity | AuthType::Dummy
    )
}

/// True when no advertised flow can be completed with only email / password / dummy.
fn has_unsupported_remaining_stage(info: &UiaaInfo) -> bool {
    if info.flows.is_empty() {
        return false;
    }
    !info.flows.iter().any(|flow| {
        flow.stages
            .iter()
            .all(|stage| info.completed.contains(stage) || is_supported_password_reset_stage(stage))
    })
}

fn auth_error_code(info: &UiaaInfo) -> Option<String> {
    info.auth_error
        .as_ref()
        .map(|body| body.kind.errcode().to_string())
}

fn validate_email(email: &str) -> Result<String, AuthError> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-empty-email",
            reason: "email is empty",
        });
    }
    if trimmed.len() > 256 || !trimmed.contains('@') {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-invalid-email",
            reason: "email is malformed",
        });
    }
    Ok(trimmed.to_owned())
}

fn validate_password_present(password: &str) -> Result<(), AuthError> {
    if password.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-empty-password",
            reason: "password is empty",
        });
    }
    if password.len() > 1024 {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-password-too-long",
            reason: "password exceeds length limit",
        });
    }
    Ok(())
}

fn parse_client_secret(raw: &str) -> Result<OwnedClientSecret, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-empty-client-secret",
            reason: "client secret is empty",
        });
    }
    <&ClientSecret>::try_from(trimmed)
        .map(|s| s.to_owned())
        .map_err(|_| AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-invalid-client-secret",
            reason: "client secret is malformed",
        })
}

fn parse_session_id(raw: &str) -> Result<OwnedSessionId, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-empty-sid",
            reason: "session id is empty",
        });
    }
    OwnedSessionId::try_from(trimmed).map_err(|_| AuthError::InvalidInput {
        diagnostic_id: "v-auth.4-invalid-sid",
        reason: "session id is malformed",
    })
}

/// Map HTTP/SDK password-reset errors without embedding secrets or raw bodies.
pub(crate) fn map_password_reset_http_error(err: HttpError) -> AuthError {
    map_password_reset_sdk_error(SdkError::from(err))
}

pub(crate) fn map_password_reset_sdk_error(err: SdkError) -> AuthError {
    if err.as_uiaa_response().is_some() {
        return AuthError::InteractiveAuthRequired {
            diagnostic_id: "v-auth.4-password-reset-uiaa",
        };
    }
    // Reuse login classifier (privacy-safe structural scan).
    let mapped = map_login_sdk_error(err);
    match mapped {
        AuthError::AuthenticationRejected { .. } => AuthError::AuthenticationRejected {
            diagnostic_id: "v-auth.4-password-reset-rejected",
        },
        AuthError::RateLimited { retry_after_ms, .. } => AuthError::RateLimited {
            diagnostic_id: "v-auth.4-password-reset-rate-limited",
            retry_after_ms,
        },
        AuthError::Connectivity { .. } => AuthError::Connectivity {
            diagnostic_id: "v-auth.4-password-reset-connectivity",
        },
        AuthError::HomeserverUnavailable { .. } => AuthError::HomeserverUnavailable {
            diagnostic_id: "v-auth.4-password-reset-homeserver-unavailable",
        },
        AuthError::UserDeactivated { .. } => AuthError::UserDeactivated {
            diagnostic_id: "v-auth.4-password-reset-user-deactivated",
        },
        AuthError::UnsupportedCapability { .. } => AuthError::UnsupportedCapability {
            diagnostic_id: "v-auth.4-password-reset-unsupported",
        },
        other => other,
    }
}

/// Derive a synthetic account identity for ephemeral unauthenticated password-reset clients.
///
/// Isolation is per homeserver host only — never stores session secrets.
pub fn password_reset_ephemeral_user_id(homeserver_url: &str) -> Result<String, AuthError> {
    let host = host_label_from_homeserver(homeserver_url)?;
    // Matrix localpart-safe label; path isolation relies on AccountIdentity digest.
    Ok(format!("@__password_reset__:{host}"))
}

fn host_label_from_homeserver(homeserver_url: &str) -> Result<String, AuthError> {
    let trimmed = homeserver_url.trim().trim_end_matches('/');
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if host.is_empty() || host.contains(['@', '\\', '\0', ' ']) {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4-invalid-homeserver-host",
            reason: "homeserver host is invalid",
        });
    }
    // Path-safe: colons (ports) are valid in Matrix server names.
    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::api::client::uiaa::AuthFlow;

    #[test]
    fn validates_email_and_password_inputs() {
        assert!(validate_email("").is_err());
        assert!(validate_email("not-an-email").is_err());
        assert_eq!(validate_email("  a@b.co  ").unwrap(), "a@b.co");
        assert!(validate_password_present("").is_err());
        assert!(validate_password_present("x").is_ok());
    }

    #[test]
    fn parses_client_secret_and_sid() {
        assert!(parse_client_secret("").is_err());
        assert!(parse_client_secret("bad secret with spaces!").is_err());
        assert!(parse_client_secret("abcdef0123456789abcdef0123456789").is_ok());
        assert!(parse_session_id("").is_err());
        assert!(parse_session_id("sid_ok-1").is_ok());
    }

    #[test]
    fn ephemeral_user_id_from_homeserver() {
        assert_eq!(
            password_reset_ephemeral_user_id("https://matrix.example.org/").unwrap(),
            "@__password_reset__:matrix.example.org"
        );
        assert_eq!(
            password_reset_ephemeral_user_id("http://localhost:8008").unwrap(),
            "@__password_reset__:localhost:8008"
        );
        assert!(password_reset_ephemeral_user_id("").is_err());
    }

    #[test]
    fn password_stage_detection() {
        let mut info = UiaaInfo::new(vec![AuthFlow::new(vec![
            AuthType::EmailIdentity,
            AuthType::Password,
        ])]);
        assert!(password_stage_required(&info));
        info.completed.push(AuthType::Password);
        assert!(!password_stage_required(&info));
    }

    #[test]
    fn email_not_verified_maps_errcode_without_body() {
        // Flattened UIAA auth_error fields — deserialize rather than constructing
        // ruma_common::StandardErrorBody across crate version boundaries.
        let info: UiaaInfo = serde_json::from_str(
            r#"{
                "flows": [{"stages": ["m.login.email.identity"]}],
                "session": "sess",
                "errcode": "M_UNAUTHORIZED",
                "error": "raw should not leak via product outcome message path"
            }"#,
        )
        .expect("uiaa fixture");
        let outcome = email_not_verified_outcome(&info).expect("mapped");
        match outcome {
            PasswordResetOutcome::EmailNotVerified {
                session,
                error_code,
                error_message,
            } => {
                assert_eq!(session.as_deref(), Some("sess"));
                assert_eq!(error_code.as_deref(), Some("M_UNAUTHORIZED"));
                assert!(error_message.is_some());
                assert!(!error_message.unwrap().contains("raw should not leak"));
            }
            PasswordResetOutcome::Complete => panic!("expected email not verified"),
        }
    }

    #[test]
    fn unsupported_stage_detection() {
        let info = UiaaInfo::new(vec![AuthFlow::new(vec![
            AuthType::EmailIdentity,
            AuthType::ReCaptcha,
        ])]);
        assert!(has_unsupported_remaining_stage(&info));

        let only_supported = UiaaInfo::new(vec![AuthFlow::new(vec![
            AuthType::EmailIdentity,
            AuthType::Password,
        ])]);
        assert!(!has_unsupported_remaining_stage(&only_supported));
    }

    #[test]
    fn outcome_serializes_camel_case_fields() {
        let outcome = PasswordResetOutcome::EmailNotVerified {
            session: Some("s".into()),
            error_code: Some("M_UNAUTHORIZED".into()),
            error_message: Some("Email has not been verified yet, or the verification expired."),
        };
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("\"status\":\"email_not_verified\""));
        assert!(json.contains("errorCode"));
        assert!(json.contains("errorMessage"));
        assert!(!json.contains("error_code"));
    }
}
