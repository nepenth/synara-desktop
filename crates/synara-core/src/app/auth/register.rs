//! V-AUTH.4b — native desktop password registration via Matrix CS API.
//!
//! Owns unauthenticated registration through managed `matrix_sdk` clients +
//! Ruma request types (no raw REST strings):
//! 1. registration flow probe (`POST /register` empty → UIAA flows / disabled)
//! 2. registration email token request
//! 3. multi-stage register submit for product-supported UIAA stages
//!
//! Access/refresh tokens from a successful register stay on the host process and
//! never cross Tauri IPC. Secrets (password, client secret, registration token,
//! captcha response) are never stored on the coordinator and never appear in
//! diagnostic ids or Display text. No dual-backend.

use super::{
    has_unsupported_only_register_flows, RegisterFlowsProbe, RegisterUiaFlow,
    SUPPORTED_REGISTER_STAGES,
};
use matrix_sdk::ruma::{
    api::client::{
        account::{register, request_registration_token_via_email},
        uiaa::{
            AuthData, AuthFlow, AuthType, Dummy, ReCaptcha, RegistrationToken, Terms, UiaaInfo,
        },
    },
    assign, ClientSecret, OwnedClientSecret, OwnedSessionId, UInt,
};
use matrix_sdk::{Client, Error as SdkError, HttpError};
use serde::Serialize;
use serde_json::{json, Map, Value as JsonValue};
use zeroize::Zeroizing;

use super::error::AuthError;
use super::login::map_login_sdk_error;
use super::reset_password::{
    map_password_reset_http_error, password_reset_ephemeral_user_id, PasswordEmailTokenResult,
};

/// Host-only secrets from a completed registration (never serialized to IPC).
#[derive(Debug)]
pub struct RegisterCompleteSecrets {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
    pub access_token: Zeroizing<String>,
    pub refresh_token: Option<Zeroizing<String>>,
}

impl Drop for RegisterCompleteSecrets {
    fn drop(&mut self) {
        // Zeroizing handles token fields; clear identity strings that are not secrets.
        self.user_id.clear();
        self.device_id.clear();
        self.homeserver_url.clear();
    }
}

/// Privacy-safe UIAA challenge returned to the product UI.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUiaChallenge {
    pub session: Option<String>,
    pub flows: Vec<RegisterUiaFlow>,
    pub completed: Vec<String>,
    /// Opaque stage params (e.g. recaptcha public_key, terms policies).
    pub params: Option<JsonValue>,
    pub error_code: Option<String>,
    pub error_message: Option<&'static str>,
}

/// Host outcome of a register submit. Complete secrets never cross IPC.
#[derive(Debug)]
pub enum RegisterSubmitOutcome {
    Complete(RegisterCompleteSecrets),
    UiaRequired(RegisterUiaChallenge),
}

/// Stage payload from the product UI (typed; no free-form auth dict).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegisterAuthStage {
    /// First attempt: only the UIAA session from the flow probe.
    #[serde(rename_all = "camelCase")]
    SessionOnly { session: Option<String> },
    #[serde(rename_all = "camelCase")]
    Dummy { session: Option<String> },
    #[serde(rename_all = "camelCase")]
    Terms { session: Option<String> },
    #[serde(rename_all = "camelCase")]
    RegistrationToken {
        token: String,
        session: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Recaptcha {
        response: String,
        session: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    EmailIdentity {
        sid: String,
        client_secret: String,
        session: Option<String>,
    },
}

/// Request a registration email token via Ruma `request_registration_token_via_email`.
pub async fn request_register_email_token(
    client: &Client,
    email: &str,
    client_secret: &str,
    send_attempt: u32,
) -> Result<PasswordEmailTokenResult, AuthError> {
    let email = validate_email(email)?;
    let secret = parse_client_secret(client_secret)?;
    let attempt = UInt::from(send_attempt);
    let request = request_registration_token_via_email::v3::Request::new(secret, email, attempt);
    let response = client
        .send(request)
        .await
        .map_err(map_password_reset_http_error)?;
    Ok(PasswordEmailTokenResult {
        sid: response.sid.to_string(),
        submit_url: response.submit_url,
    })
}

/// Submit a registration attempt with optional UIAA stage data.
pub async fn register_submit(
    client: &Client,
    username: &str,
    password: &str,
    device_display_name: &str,
    stage: RegisterAuthStage,
) -> Result<RegisterSubmitOutcome, AuthError> {
    let username = validate_username(username)?;
    validate_password_present(password)?;
    validate_device_display_name(device_display_name)?;
    let auth = auth_data_from_stage(stage)?;

    let request = assign!(register::v3::Request::new(), {
        username: Some(username),
        password: Some(password.to_owned()),
        initial_device_display_name: Some(device_display_name.to_owned()),
        auth: Some(auth),
        refresh_token: true,
    });

    match client.send(request).await {
        Ok(response) => {
            let access_token = response.access_token.ok_or(AuthError::SdkInvariant {
                diagnostic_id: "v-auth.4b-register-missing-access-token",
            })?;
            let device_id = response.device_id.ok_or(AuthError::SdkInvariant {
                diagnostic_id: "v-auth.4b-register-missing-device-id",
            })?;
            Ok(RegisterSubmitOutcome::Complete(RegisterCompleteSecrets {
                user_id: response.user_id.to_string(),
                device_id: device_id.to_string(),
                homeserver_url: client.homeserver().to_string(),
                access_token: Zeroizing::new(access_token),
                refresh_token: response.refresh_token.map(Zeroizing::new),
            }))
        }
        Err(err) => {
            if let Some(info) = err.as_uiaa_response() {
                return Ok(RegisterSubmitOutcome::UiaRequired(uia_challenge_from_info(
                    info,
                )?));
            }
            Err(map_register_http_error(err))
        }
    }
}

/// Synthetic account identity for ephemeral unauthenticated registration clients.
pub fn register_ephemeral_user_id(homeserver_url: &str) -> Result<String, AuthError> {
    // Reuse host-label parsing from password-reset (privacy-safe path isolation).
    let synthetic = password_reset_ephemeral_user_id(homeserver_url)?;
    Ok(synthetic.replacen("__password_reset__", "__register__", 1))
}

fn auth_data_from_stage(stage: RegisterAuthStage) -> Result<AuthData, AuthError> {
    match stage {
        RegisterAuthStage::SessionOnly { session } => {
            // Fallback acknowledgement carries session for the first challenge continue.
            let session = session.unwrap_or_default();
            if session.is_empty() {
                // Empty auth object via Dummy without type is not valid; use Dummy with no session.
                return Ok(AuthData::Dummy(Dummy::new()));
            }
            Ok(AuthData::fallback_acknowledgement(session))
        }
        RegisterAuthStage::Dummy { session } => {
            let mut dummy = Dummy::new();
            dummy.session = session;
            Ok(AuthData::Dummy(dummy))
        }
        RegisterAuthStage::Terms { session } => {
            let mut terms = Terms::new();
            terms.session = session;
            Ok(AuthData::Terms(terms))
        }
        RegisterAuthStage::RegistrationToken { token, session } => {
            let token = token.trim();
            if token.is_empty() {
                return Err(AuthError::InvalidInput {
                    diagnostic_id: "v-auth.4b-empty-registration-token",
                    reason: "registration token is empty",
                });
            }
            if token.len() > 512 {
                return Err(AuthError::InvalidInput {
                    diagnostic_id: "v-auth.4b-registration-token-too-long",
                    reason: "registration token exceeds length limit",
                });
            }
            let mut reg = RegistrationToken::new(token.to_owned());
            reg.session = session;
            Ok(AuthData::RegistrationToken(reg))
        }
        RegisterAuthStage::Recaptcha { response, session } => {
            let response = response.trim();
            if response.is_empty() {
                return Err(AuthError::InvalidInput {
                    diagnostic_id: "v-auth.4b-empty-recaptcha",
                    reason: "recaptcha response is empty",
                });
            }
            if response.len() > 4096 {
                return Err(AuthError::InvalidInput {
                    diagnostic_id: "v-auth.4b-recaptcha-too-long",
                    reason: "recaptcha response exceeds length limit",
                });
            }
            let mut captcha = ReCaptcha::new(response.to_owned());
            captcha.session = session;
            Ok(AuthData::ReCaptcha(captcha))
        }
        RegisterAuthStage::EmailIdentity {
            sid,
            client_secret,
            session,
        } => {
            // Validate sid/secret shape without constructing non_exhaustive email structs.
            let _sid = parse_session_id(&sid)?;
            let _secret = parse_client_secret(&client_secret)?;
            let mut data = Map::new();
            data.insert(
                "threepid_creds".into(),
                json!({
                    "sid": sid.trim(),
                    "client_secret": client_secret.trim(),
                }),
            );
            AuthData::new("m.login.email.identity", session, data).map_err(|_| {
                AuthError::InvalidInput {
                    diagnostic_id: "v-auth.4b-email-auth-encode",
                    reason: "failed to encode email identity auth",
                }
            })
        }
    }
}

fn uia_challenge_from_info(info: &UiaaInfo) -> Result<RegisterUiaChallenge, AuthError> {
    // Fail closed if every advertised flow requires an unsupported stage.
    if has_unsupported_only_flows(info) {
        return Err(AuthError::UnsupportedCapability {
            diagnostic_id: "v-auth.4b-register-unsupported-uia-stage",
        });
    }
    Ok(RegisterUiaChallenge {
        session: info.session.clone(),
        flows: info.flows.iter().map(map_flow).collect(),
        completed: info
            .completed
            .iter()
            .map(|stage| stage.to_string())
            .collect(),
        params: info
            .params
            .as_ref()
            .and_then(|raw| serde_json::from_str(raw.get()).ok()),
        error_code: info
            .auth_error
            .as_ref()
            .map(|body| body.kind.errcode().to_string()),
        error_message: static_uia_error_message(info),
    })
}

fn map_flow(flow: &AuthFlow) -> RegisterUiaFlow {
    RegisterUiaFlow {
        stages: flow.stages.iter().map(|s| s.to_string()).collect(),
    }
}

fn has_unsupported_only_flows(info: &UiaaInfo) -> bool {
    let flows: Vec<_> = info.flows.iter().map(map_flow).collect();
    let completed: Vec<_> = info.completed.iter().map(ToString::to_string).collect();
    has_unsupported_only_register_flows(&flows, &completed)
}

fn static_uia_error_message(info: &UiaaInfo) -> Option<&'static str> {
    let code = info
        .auth_error
        .as_ref()
        .map(|body| body.kind.errcode().to_string())?;
    Some(match code.as_str() {
        "M_UNAUTHORIZED" | "M_FORBIDDEN" => {
            "Registration authentication was rejected or is incomplete."
        }
        "M_THREEPID_AUTH_FAILED" | "M_THREEPID_IN_USE" | "M_THREEPID_DENIED" => {
            "Email verification failed. Request a new verification email."
        }
        _ => "Registration authentication was rejected.",
    })
}

fn map_register_http_error(err: HttpError) -> AuthError {
    let sdk_err = SdkError::from(err);
    if let Some(kind) = sdk_err.client_api_error_kind() {
        let k = format!("{kind:?}").to_ascii_lowercase();
        if k.starts_with("userinuse") {
            return AuthError::AuthenticationRejected {
                diagnostic_id: "v-auth.4b-register-user-taken",
            };
        }
        if k.starts_with("invalidusername") {
            return AuthError::InvalidInput {
                diagnostic_id: "v-auth.4b-register-user-invalid",
                reason: "username is invalid",
            };
        }
        if k.starts_with("exclusive") {
            return AuthError::AuthenticationRejected {
                diagnostic_id: "v-auth.4b-register-user-exclusive",
            };
        }
        if k.starts_with("weakpassword") {
            return AuthError::AuthenticationRejected {
                diagnostic_id: "v-auth.4b-register-password-weak",
            };
        }
        if k.contains("password") && k.contains("short") {
            return AuthError::AuthenticationRejected {
                diagnostic_id: "v-auth.4b-register-password-short",
            };
        }
        if k.starts_with("forbidden") {
            return AuthError::AuthenticationRejected {
                diagnostic_id: "v-auth.4b-register-forbidden",
            };
        }
        if k.starts_with("limitexceeded") {
            return AuthError::RateLimited {
                diagnostic_id: "v-auth.4b-register-rate-limited",
                retry_after_ms: None,
            };
        }
    }
    let mapped = map_login_sdk_error(sdk_err);
    match mapped {
        AuthError::AuthenticationRejected { .. } => AuthError::AuthenticationRejected {
            diagnostic_id: "v-auth.4b-register-rejected",
        },
        AuthError::RateLimited { retry_after_ms, .. } => AuthError::RateLimited {
            diagnostic_id: "v-auth.4b-register-rate-limited",
            retry_after_ms,
        },
        AuthError::Connectivity { .. } => AuthError::Connectivity {
            diagnostic_id: "v-auth.4b-register-connectivity",
        },
        AuthError::HomeserverUnavailable { .. } => AuthError::HomeserverUnavailable {
            diagnostic_id: "v-auth.4b-register-homeserver-unavailable",
        },
        AuthError::UnsupportedCapability { .. } => AuthError::UnsupportedCapability {
            diagnostic_id: "v-auth.4b-register-unsupported",
        },
        other => other,
    }
}

fn validate_username(username: &str) -> Result<String, AuthError> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-empty-username",
            reason: "username is empty",
        });
    }
    if trimmed.len() > 255 || trimmed.contains([':', ' ', '@', '\\', '\0']) {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-invalid-username",
            reason: "username is malformed",
        });
    }
    Ok(trimmed.to_owned())
}

fn validate_password_present(password: &str) -> Result<(), AuthError> {
    if password.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-empty-password",
            reason: "password is empty",
        });
    }
    if password.len() > 1024 {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-password-too-long",
            reason: "password exceeds length limit",
        });
    }
    Ok(())
}

fn validate_device_display_name(name: &str) -> Result<(), AuthError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-empty-device-display-name",
            reason: "device display name is empty",
        });
    }
    if trimmed.len() > 128 {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-device-display-name-too-long",
            reason: "device display name exceeds length limit",
        });
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<String, AuthError> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-empty-email",
            reason: "email is empty",
        });
    }
    if trimmed.len() > 256 || !trimmed.contains('@') {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-invalid-email",
            reason: "email is malformed",
        });
    }
    Ok(trimmed.to_owned())
}

fn parse_client_secret(raw: &str) -> Result<OwnedClientSecret, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-empty-client-secret",
            reason: "client secret is empty",
        });
    }
    <&ClientSecret>::try_from(trimmed)
        .map(|s| s.to_owned())
        .map_err(|_| AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-invalid-client-secret",
            reason: "client secret is malformed",
        })
}

fn parse_session_id(raw: &str) -> Result<OwnedSessionId, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "v-auth.4b-empty-sid",
            reason: "session id is empty",
        });
    }
    OwnedSessionId::try_from(trimmed).map_err(|_| AuthError::InvalidInput {
        diagnostic_id: "v-auth.4b-invalid-sid",
        reason: "session id is malformed",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_register_inputs() {
        assert!(validate_username("").is_err());
        assert!(validate_username("bad:name").is_err());
        assert_eq!(validate_username("  alice  ").unwrap(), "alice");
        assert!(validate_password_present("").is_err());
        assert!(validate_password_present("x").is_ok());
        assert!(validate_device_display_name("").is_err());
        assert!(validate_device_display_name("Synara macOS").is_ok());
        assert!(validate_email("a@b.co").is_ok());
    }

    #[test]
    fn ephemeral_register_user_id() {
        assert_eq!(
            register_ephemeral_user_id("https://matrix.example.org/").unwrap(),
            "@__register__:matrix.example.org"
        );
    }

    #[test]
    fn auth_stage_builders_do_not_panic() {
        let dummy = auth_data_from_stage(RegisterAuthStage::Dummy {
            session: Some("s".into()),
        })
        .unwrap();
        assert_eq!(dummy.session(), Some("s"));

        let terms = auth_data_from_stage(RegisterAuthStage::Terms {
            session: Some("s".into()),
        })
        .unwrap();
        assert_eq!(terms.session(), Some("s"));

        let token = auth_data_from_stage(RegisterAuthStage::RegistrationToken {
            token: "invite".into(),
            session: Some("s".into()),
        })
        .unwrap();
        assert_eq!(token.session(), Some("s"));

        assert!(auth_data_from_stage(RegisterAuthStage::RegistrationToken {
            token: "".into(),
            session: None,
        })
        .is_err());
    }

    #[test]
    fn unsupported_stage_detection() {
        let info = UiaaInfo::new(vec![AuthFlow::new(vec![AuthType::Msisdn])]);
        assert!(has_unsupported_only_flows(&info));

        let ok = UiaaInfo::new(vec![AuthFlow::new(vec![
            AuthType::Terms,
            AuthType::ReCaptcha,
            AuthType::Dummy,
        ])]);
        assert!(!has_unsupported_only_flows(&ok));
    }

    #[test]
    fn uia_challenge_serializes_camel_case() {
        let challenge = RegisterUiaChallenge {
            session: Some("sess".into()),
            flows: vec![RegisterUiaFlow {
                stages: vec!["m.login.dummy".into()],
            }],
            completed: vec![],
            params: None,
            error_code: Some("M_UNAUTHORIZED".into()),
            error_message: Some("Registration authentication was rejected."),
        };
        let json = serde_json::to_string(&challenge).unwrap();
        assert!(json.contains("errorCode"));
        assert!(json.contains("errorMessage"));
        assert!(!json.contains("error_code"));
    }

    #[test]
    fn supported_stages_cover_product_list() {
        assert!(SUPPORTED_REGISTER_STAGES.contains(&"m.login.recaptcha"));
        assert!(SUPPORTED_REGISTER_STAGES.contains(&"m.login.email.identity"));
        assert!(SUPPORTED_REGISTER_STAGES.contains(&"m.login.registration_token"));
    }

    #[test]
    fn probe_and_submit_outcome_tags() {
        let probe = RegisterFlowsProbe::RegistrationDisabled;
        let json = serde_json::to_string(&probe).unwrap();
        assert!(json.contains("registration_disabled"));
    }
}
