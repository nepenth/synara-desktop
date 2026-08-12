//! Stateless auth-probe bridge for the desktop Tauri command surface.
//!
//! These are deliberately the only P3.1 commands routed through `Core` in
//! this slice. The renderer keeps invoking the existing Tauri commands with
//! `{ homeserverUrl }`; this adapter creates the Core envelope and unwraps the
//! known Core DTO back into the pre-existing Tauri response shape.

use serde::de::DeserializeOwned;
use synara_core::app::auth::{MatrixLoginFlowsResponse, RegisterFlowsProbe};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

/// Stateless probes have no live session. Zero is a valid JSON-safe core
/// session generation (`CommandEnvelope::validate` verifies it), so it is the
/// neutral envelope generation for both read-only requests.
const STATELESS_SESSION_GENERATION: u64 = 0;

const LOGIN_FLOWS_COMMAND: &str = "matrix_login_flows";
const REGISTER_FLOWS_COMMAND: &str = "matrix_register_flows";

/// Route the existing `matrix_login_flows` Tauri input through the managed
/// Core. The payload intentionally has exactly the React-facing camel-case
/// field; no credential or UIAA data can enter this bridge.
pub(crate) async fn login_flows(
    core: &Core,
    homeserver_url: String,
) -> Result<MatrixLoginFlowsResponse, MatrixAuthCommandError> {
    invoke_probe(
        core,
        LOGIN_FLOWS_COMMAND,
        homeserver_url,
        map_login_flows_core_error,
        login_flows_invalid_core_response,
    )
    .await
}

/// Route the existing `matrix_register_flows` Tauri input through the managed
/// Core. It is intentionally only the empty registration-flow probe; submit,
/// email-token, and UIAA-continuation commands remain desktop-owned.
pub(crate) async fn register_flows(
    core: &Core,
    homeserver_url: String,
) -> Result<RegisterFlowsProbe, MatrixAuthCommandError> {
    invoke_probe(
        core,
        REGISTER_FLOWS_COMMAND,
        homeserver_url,
        map_register_flows_core_error,
        register_flows_invalid_core_response,
    )
    .await
}

async fn invoke_probe<Response>(
    core: &Core,
    command: &'static str,
    homeserver_url: String,
    map_core_error: fn(MatrixIpcError) -> MatrixAuthCommandError,
    invalid_response: fn() -> MatrixAuthCommandError,
) -> Result<Response, MatrixAuthCommandError>
where
    Response: DeserializeOwned,
{
    let request = CommandEnvelope {
        command: command.to_owned(),
        session_generation: STATELESS_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({ "homeserverUrl": homeserver_url }),
    };

    // `Core::command` validates this envelope before dispatch. In particular,
    // the neutral generation is within the shared JS-safe wire-counter range.
    let response = core.command(request).await.map_err(map_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| invalid_response())
}

/// Map only the stable Core category. Core messages/diagnostics are never
/// reflected here: they could contain a URL, UIAA body, or other private data.
pub(crate) fn map_login_flows_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let (code, message, diagnostic_id) = match error.category {
        MatrixIpcErrorCategory::SdkInvariant => (
            "InvalidRequest",
            "The login-flow discovery request is invalid.",
            "snc-p3-1-login-flows-invalid-request",
        ),
        MatrixIpcErrorCategory::RateLimited => (
            "RateLimited",
            "Login-flow discovery was rate limited.",
            "snc-p3-1-login-flows-rate-limited",
        ),
        MatrixIpcErrorCategory::Connectivity | MatrixIpcErrorCategory::HomeserverUnavailable => (
            "InvalidServer",
            "The Matrix homeserver is unavailable.",
            "snc-p3-1-login-flows-homeserver-unavailable",
        ),
        MatrixIpcErrorCategory::UnsupportedCapability => (
            "Unsupported",
            "The homeserver returned unsupported login-flow data.",
            "snc-p3-1-login-flows-unsupported",
        ),
        _ => (
            "Unknown",
            "Native login-flow discovery failed.",
            "snc-p3-1-login-flows-core-failed",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

/// Map only the stable Core category. This deliberately does not use
/// registration submit's domain mapper because this command is just the
/// credential-free flow probe.
pub(crate) fn map_register_flows_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let (code, message, diagnostic_id) = match error.category {
        MatrixIpcErrorCategory::SdkInvariant => (
            "InvalidRequest",
            "The registration request is invalid.",
            "snc-p3-1-register-flows-invalid-request",
        ),
        MatrixIpcErrorCategory::RateLimited => (
            "RateLimited",
            "The registration request was rate limited.",
            "snc-p3-1-register-flows-rate-limited",
        ),
        MatrixIpcErrorCategory::Connectivity | MatrixIpcErrorCategory::HomeserverUnavailable => (
            "InvalidServer",
            "The Matrix homeserver is unavailable.",
            "snc-p3-1-register-flows-homeserver-unavailable",
        ),
        MatrixIpcErrorCategory::UnsupportedCapability => (
            "Unsupported",
            "The homeserver requires an unsupported registration stage.",
            "snc-p3-1-register-flows-unsupported",
        ),
        _ => (
            "Unknown",
            "Native registration failed.",
            "snc-p3-1-register-flows-core-failed",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

fn login_flows_invalid_core_response() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native login-flow discovery failed.",
        "snc-p3-1-login-flows-response-invalid",
    )
}

fn register_flows_invalid_core_response() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native registration failed.",
        "snc-p3-1-register-flows-response-invalid",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{Platform, PlatformStatus, SecretVault, UnavailableSecretVault};
    use synara_core::transport::{CommandFuture, CommandRegistry, MatrixIpcEnvelope};

    use super::*;

    struct TestPlatform;

    impl Platform for TestPlatform {
        fn emit(&self, _envelope: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }

        fn http_user_agent(&self) -> String {
            "Synara-Desktop-Bridge-Test/1.0".to_owned()
        }

        fn sync_status(&self) -> synara_core::platform::SyncStatusFuture<'_> {
            Box::pin(async {
                Ok(synara_core::platform::PlatformSyncStatus::new(
                    synara_core::app::sync::SyncReadiness::Unconfigured,
                    0,
                    false,
                    None,
                    None,
                )
                .expect("unconfigured status is a valid string-free projection"))
            })
        }

        fn crypto_status(&self) -> synara_core::platform::CryptoStatusFuture<'_> {
            Box::pin(async {
                Ok(synara_core::platform::PlatformCryptoStatus::new(
                    0,
                    false,
                    synara_core::platform::PlatformCryptoCrossSigningState::Unavailable,
                )
                .expect("unavailable is a valid string-free crypto projection"))
            })
        }

        fn cross_signing_status(&self) -> synara_core::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async {
                Err(synara_core::platform::PlatformCrossSigningStatusError::NoSession)
            })
        }

        fn media_config(&self) -> synara_core::platform::MediaConfigFuture<'_> {
            Box::pin(async {
                Ok(synara_core::platform::PlatformMediaConfig::new(0)
                    .expect("zero is a valid closed media projection"))
            })
        }

        fn notify(&self, _candidate: NotificationCandidate) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    fn core_returning(
        command: &'static str,
        response_payload: serde_json::Value,
        forwarded: Arc<Mutex<Vec<CommandEnvelope>>>,
    ) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(command, move |_state, request| -> CommandFuture {
                forwarded.lock().expect("test capture lock").push(request);
                let response_payload = response_payload.clone();
                Box::pin(async move { Ok(response_payload) })
            })
            .expect("test command is in the desktop census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    #[test]
    fn stateless_probe_envelope_uses_a_valid_neutral_generation() {
        let envelope = CommandEnvelope {
            command: LOGIN_FLOWS_COMMAND.to_owned(),
            session_generation: STATELESS_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "homeserverUrl": "https://matrix.example" }),
        };
        assert!(envelope.validate().is_ok());
    }

    #[tokio::test]
    async fn login_flows_forwards_the_exact_payload_and_response() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = serde_json::json!({
            "flows": [{
                "kind": "password",
                "matrixType": "m.login.password",
                "getLoginToken": true
            }]
        });
        let core = core_returning(LOGIN_FLOWS_COMMAND, payload.clone(), Arc::clone(&forwarded));

        let response = login_flows(&core, "https://matrix.example".to_owned())
            .await
            .expect("known Core response is a desktop DTO");

        assert_eq!(serde_json::to_value(response).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: LOGIN_FLOWS_COMMAND.to_owned(),
                session_generation: STATELESS_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "homeserverUrl": "https://matrix.example" }),
            }]
        );
    }

    #[tokio::test]
    async fn register_flows_forwards_the_exact_payload_and_response() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = serde_json::json!({
            "status": "flow_required",
            "session": "opaque-uia-session",
            "flows": [{ "stages": ["m.login.terms"] }],
            "completed": [],
            "params": { "m.login.terms": { "policies": [] } }
        });
        let core = core_returning(
            REGISTER_FLOWS_COMMAND,
            payload.clone(),
            Arc::clone(&forwarded),
        );

        let response = register_flows(&core, "https://matrix.example".to_owned())
            .await
            .expect("known Core response is a desktop DTO");

        assert_eq!(serde_json::to_value(response).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: REGISTER_FLOWS_COMMAND.to_owned(),
                session_generation: STATELESS_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "homeserverUrl": "https://matrix.example" }),
            }]
        );
    }

    #[test]
    fn core_error_mapping_is_static_and_does_not_reflect_private_text() {
        let private_text =
            "https://private.example/_matrix password=secret token=secret UIAA params/body";
        for (category, login_code, login_diagnostic, register_code, register_diagnostic) in [
            (
                MatrixIpcErrorCategory::SdkInvariant,
                "InvalidRequest",
                "snc-p3-1-login-flows-invalid-request",
                "InvalidRequest",
                "snc-p3-1-register-flows-invalid-request",
            ),
            (
                MatrixIpcErrorCategory::RateLimited,
                "RateLimited",
                "snc-p3-1-login-flows-rate-limited",
                "RateLimited",
                "snc-p3-1-register-flows-rate-limited",
            ),
            (
                MatrixIpcErrorCategory::Connectivity,
                "InvalidServer",
                "snc-p3-1-login-flows-homeserver-unavailable",
                "InvalidServer",
                "snc-p3-1-register-flows-homeserver-unavailable",
            ),
            (
                MatrixIpcErrorCategory::HomeserverUnavailable,
                "InvalidServer",
                "snc-p3-1-login-flows-homeserver-unavailable",
                "InvalidServer",
                "snc-p3-1-register-flows-homeserver-unavailable",
            ),
            (
                MatrixIpcErrorCategory::UnsupportedCapability,
                "Unsupported",
                "snc-p3-1-login-flows-unsupported",
                "Unsupported",
                "snc-p3-1-register-flows-unsupported",
            ),
            (
                MatrixIpcErrorCategory::Unknown,
                "Unknown",
                "snc-p3-1-login-flows-core-failed",
                "Unknown",
                "snc-p3-1-register-flows-core-failed",
            ),
        ] {
            let error = MatrixIpcError {
                category,
                message: Some(private_text.to_owned()),
                diagnostic_id: Some(private_text.to_owned()),
                retry_after_ms: Some(1),
                request_id: Some(private_text.to_owned()),
            };
            let login = map_login_flows_core_error(error.clone());
            let register = map_register_flows_core_error(error);
            assert_eq!(login.code, login_code);
            assert_eq!(login.diagnostic_id, login_diagnostic);
            assert_eq!(register.code, register_code);
            assert_eq!(register.diagnostic_id, register_diagnostic);
            for mapped in [login, register] {
                let serialized = serde_json::to_string(&mapped).unwrap();
                for forbidden in ["private.example", "password", "secret", "token", "UIAA"] {
                    assert!(!serialized.contains(forbidden));
                }
            }
        }
    }

    fn command_body<'a>(source: &'a str, command: &str) -> &'a str {
        let signature = format!("pub async fn {command}");
        let start = source.find(&signature).expect("command must exist");
        let body_start = source[start..]
            .find('{')
            .map(|offset| start + offset)
            .expect("command must have body");
        let mut depth = 0_u32;
        for (offset, byte) in source[body_start..].bytes().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &source[start..=body_start + offset];
                    }
                }
                _ => {}
            }
        }
        panic!("command body must close");
    }

    #[test]
    fn auth_probe_commands_have_no_direct_auth_transport_call() {
        let product_commands = include_str!("../matrix/auth/product_commands.rs");
        for (command, bridge_call) in [
            (
                LOGIN_FLOWS_COMMAND,
                "crate::bridge::auth_probes::login_flows",
            ),
            (
                REGISTER_FLOWS_COMMAND,
                "crate::bridge::auth_probes::register_flows",
            ),
        ] {
            let body = command_body(product_commands, command);
            assert!(
                body.contains(bridge_call),
                "{command} must delegate to Core"
            );
            for forbidden in [
                "HttpLoginFlowTransport",
                "HttpRegisterFlowTransport",
                "discover_login_flows",
                "probe_register_flows",
                "new_with_user_agent",
            ] {
                assert!(
                    !body.contains(forbidden),
                    "{command} must not call auth transport directly: {forbidden}"
                );
            }
        }
    }
}
