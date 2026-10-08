//! Stateless auth-probe bridge for the desktop Tauri command surface.
//!
//! These are deliberately the only P3.1 commands routed through `Core` in
//! this slice. The renderer keeps invoking the existing Tauri commands with
//! `{ homeserverUrl }`; this adapter creates the Core envelope and unwraps the
//! known Core DTO back into the pre-existing Tauri response shape.

use synara_core::app::auth::{MatrixLoginFlowsResponse, RegisterFlowsProbe};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

/// Route the existing `matrix_login_flows` Tauri input through the managed
/// Core. The payload intentionally has exactly the React-facing camel-case
/// field; no credential or UIAA data can enter this bridge.
pub(crate) async fn login_flows(
    core: &Core,
    homeserver_url: String,
) -> Result<MatrixLoginFlowsResponse, MatrixAuthCommandError> {
    core.login_flows(synara_core::core_api::MatrixLoginFlowsRequest { homeserver_url })
        .await
        .map_err(map_login_flows_core_error)
}

/// Route the existing `matrix_register_flows` Tauri input through the managed
/// Core. It is intentionally only the empty registration-flow probe; submit,
/// email-token, and UIAA-continuation commands remain desktop-owned.
pub(crate) async fn register_flows(
    core: &Core,
    homeserver_url: String,
) -> Result<RegisterFlowsProbe, MatrixAuthCommandError> {
    core.register_flows(synara_core::core_api::MatrixRegisterFlowsRequest { homeserver_url })
        .await
        .map_err(map_register_flows_core_error)
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

#[cfg(test)]
mod tests {
    use super::*;

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
                "matrix_login_flows",
                "crate::bridge::auth_probes::login_flows",
            ),
            (
                "matrix_register_flows",
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
