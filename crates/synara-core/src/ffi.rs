//! UniFFI-facing, credential-free Matrix login-flow discovery (P4-2).
//!
//! This module owns the intentionally small translation from the shared auth
//! domain to the project-owned UniFFI surface. It exposes neither sessions nor
//! credentials and deliberately reduces all failures to fixed public values.

use crate::app::auth::{discover_login_flows, AuthError, HttpLoginFlowTransport, LoginFlow};

/// One Matrix login flow exposed through UniFFI.
///
/// The fields mirror the shared auth domain exactly while preserving its public
/// string discriminators for Swift. `get_login_token` is present only when the
/// homeserver advertised that capability; it is metadata, not a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFlowDto {
    pub kind: String,
    pub matrix_type: String,
    pub get_login_token: Option<bool>,
}

impl From<LoginFlow> for LoginFlowDto {
    fn from(flow: LoginFlow) -> Self {
        Self {
            kind: flow.kind.as_str().to_owned(),
            matrix_type: flow.matrix_type,
            get_login_token: flow.get_login_token,
        }
    }
}

/// Fixed, privacy-safe failure returned by [`login_flows`].
///
/// Every field is selected from static source constants. No URL, response
/// body, header, network-library diagnostic, credential, or token can reach
/// this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginFlowsError {
    ProbeFailed {
        category: String,
        code: String,
        description: String,
    },
}

impl std::fmt::Display for LoginFlowsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Do not format category/code: they are structured FFI fields and
            // the display representation must stay input-independent too.
            Self::ProbeFailed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for LoginFlowsError {}

impl From<AuthError> for LoginFlowsError {
    fn from(error: AuthError) -> Self {
        Self::ProbeFailed {
            category: error.category().as_str().to_owned(),
            code: error.diagnostic_id().to_owned(),
            description: ffi_error_description(&error).to_owned(),
        }
    }
}

fn ffi_error_description(error: &AuthError) -> &'static str {
    match error {
        AuthError::InvalidInput { .. } => "The homeserver URL is invalid.",
        AuthError::Connectivity { .. } => "The homeserver could not be reached.",
        AuthError::HomeserverUnavailable { .. } => "The homeserver is unavailable.",
        AuthError::WellKnownNotFound { .. } | AuthError::UnsupportedCapability { .. } => {
            "The homeserver does not support login-flow discovery."
        }
        AuthError::AuthenticationRejected { .. } => "Authentication was rejected.",
        AuthError::UserDeactivated { .. } => "The account is deactivated.",
        AuthError::InteractiveAuthRequired { .. } => "Interactive authentication is required.",
        AuthError::RateLimited { .. } => "The homeserver rate limited the request.",
        AuthError::SdkInvariant { .. } => "The login-flow probe could not be completed.",
        AuthError::Unknown { .. } => "The login-flow probe failed.",
    }
}

/// List homeserver-advertised Matrix login flows through the bounded,
/// redirect-free shared-core transport.
///
/// This operation is read-only and accepts only a homeserver URL. It submits
/// no password, token, credential, UIAA payload, session, or platform callback.
pub async fn login_flows(homeserver_url: String) -> Result<Vec<LoginFlowDto>, LoginFlowsError> {
    let transport = HttpLoginFlowTransport::new().map_err(LoginFlowsError::from)?;
    let result = discover_login_flows(&homeserver_url, &transport)
        .await
        .map_err(LoginFlowsError::from)?;
    Ok(result.flows.into_iter().map(LoginFlowDto::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_login_flow_dto_maps_known_and_unknown_types_exactly() {
        let fixtures = [
            (LoginFlow::password(), "password", "m.login.password", None),
            (LoginFlow::token(true), "token", "m.login.token", Some(true)),
            (
                LoginFlow::token(false),
                "token",
                "m.login.token",
                Some(false),
            ),
            (
                LoginFlow::application_service(),
                "application_service",
                "m.login.application_service",
                None,
            ),
            (
                LoginFlow::from_matrix_parts("m.login.sso", None),
                "unknown",
                "m.login.sso",
                None,
            ),
            (
                LoginFlow::from_matrix_parts("org.example.login", Some(false)),
                "unknown",
                "org.example.login",
                Some(false),
            ),
        ];

        for (flow, kind, matrix_type, get_login_token) in fixtures {
            let dto = LoginFlowDto::from(flow);
            assert_eq!(dto.kind, kind);
            assert_eq!(dto.matrix_type, matrix_type);
            assert_eq!(dto.get_login_token, get_login_token);
        }
    }

    #[tokio::test]
    async fn ffi_login_flows_rejects_unsafe_urls_without_echoing_input() {
        for raw in [
            "https://user:secret@example.invalid",
            "https://example.invalid/%2e%2e/private?token=do-not-expose",
        ] {
            let error = login_flows(raw.to_owned())
                .await
                .expect_err("unsafe homeserver URL must fail before a request");
            let LoginFlowsError::ProbeFailed {
                category,
                code,
                description,
            } = error;
            assert_eq!(category, "sdk_invariant");
            assert_eq!(code, "p3.1-invalid-homeserver-url");
            assert_eq!(description, "The homeserver URL is invalid.");
            for value in [&category, &code, &description] {
                assert!(!value.contains(raw));
                assert!(!value.contains("secret"));
                assert!(!value.contains("do-not-expose"));
            }
        }
    }

    #[tokio::test]
    async fn uniffi_login_flows_facade_uses_bounded_loopback_transport() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind credential-free loopback stub");
        let address = listener.local_addr().expect("loopback address");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut request = vec![0_u8; 4096];
            let read = socket.read(&mut request).await.expect("read request");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(
                request.starts_with("GET /_matrix/client/v3/login HTTP/1.1\r\n"),
                "the UniFFI facade must use the bounded core login-flow route"
            );
            let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.token","get_login_token":true},{"type":"m.login.sso"}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write login-types response");
        });

        let flows = login_flows(format!("http://{address}"))
            .await
            .expect("facade must return the core transport response");
        assert_eq!(
            flows,
            vec![
                LoginFlowDto {
                    kind: "password".to_owned(),
                    matrix_type: "m.login.password".to_owned(),
                    get_login_token: None,
                },
                LoginFlowDto {
                    kind: "token".to_owned(),
                    matrix_type: "m.login.token".to_owned(),
                    get_login_token: Some(true),
                },
                LoginFlowDto {
                    kind: "unknown".to_owned(),
                    matrix_type: "m.login.sso".to_owned(),
                    get_login_token: None,
                },
            ]
        );
        server.await.expect("loopback task");
    }

    #[test]
    fn ffi_errors_expose_only_static_privacy_safe_values() {
        let error = LoginFlowsError::from(AuthError::Connectivity {
            diagnostic_id: "r0.7-http-connect",
        });
        assert_eq!(
            error,
            LoginFlowsError::ProbeFailed {
                category: "connectivity".to_owned(),
                code: "r0.7-http-connect".to_owned(),
                description: "The homeserver could not be reached.".to_owned(),
            }
        );
    }
}
