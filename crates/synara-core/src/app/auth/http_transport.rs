//! Unauthenticated Matrix auth probes through the SDK (review S18).
//!
//! The login-flow route sends only `GET /_matrix/client/v3/login`
//! (`get_login_types`); the registration-flow route sends only an empty
//! `POST /_matrix/client/v3/register`, whose UIA challenge lists the flows.
//! Each probe runs on its own memory-only, redirect-free, time-bounded client
//! ([`build_auth_probe_client`]). Credentials and raw response bodies never
//! cross the public error surface.

use std::time::Duration;

use matrix_sdk::ruma::api::client::account::register;
use matrix_sdk::ruma::api::client::session::get_login_types::v3::LoginType;
use matrix_sdk::{Client, HttpError};
use serde_json::Value;

use super::error::AuthError;
use super::input::normalize_homeserver_url;
use super::login_flow::{LoginFlow, LoginFlowTransport};
use super::register_flow::{parse_register_uiaa_json, RegisterFlowsProbe, RegisterFlowsTransport};
use crate::app::client_builder::{build_auth_probe_client, MATRIX_SDK_PIN_VERSION};

/// Default end-to-end timeout for one unauthenticated probe request.
pub const AUTH_HTTP_TIMEOUT_SECS: u64 = 15;

pub(super) fn core_user_agent() -> String {
    format!(
        "Synara-Core/{} (matrix-sdk/{MATRIX_SDK_PIN_VERSION})",
        env!("CARGO_PKG_VERSION")
    )
}

/// Live read-only probe for `GET /_matrix/client/v3/login`.
#[derive(Debug, Clone)]
pub struct HttpLoginFlowTransport {
    user_agent: String,
}

impl HttpLoginFlowTransport {
    /// Probe with Core's fallback user agent.
    pub fn new() -> Result<Self, AuthError> {
        Self::new_with_user_agent(core_user_agent())
    }

    /// Probe with a shell-owned product identifier (`Synara-Desktop/<v>`,
    /// `Synara-iOS/<v>`), as the shell's own client sends.
    pub fn new_with_user_agent(user_agent: impl Into<String>) -> Result<Self, AuthError> {
        Ok(Self {
            user_agent: user_agent.into(),
        })
    }
}

impl Default for HttpLoginFlowTransport {
    fn default() -> Self {
        Self {
            user_agent: core_user_agent(),
        }
    }
}

/// Live read-only probe for the empty registration UIAA request.
#[derive(Debug, Clone)]
pub struct HttpRegisterFlowTransport {
    user_agent: String,
}

impl HttpRegisterFlowTransport {
    /// Probe with Core's fallback user agent.
    pub fn new() -> Result<Self, AuthError> {
        Self::new_with_user_agent(core_user_agent())
    }

    /// Probe with a shell-owned product identifier.
    pub fn new_with_user_agent(user_agent: impl Into<String>) -> Result<Self, AuthError> {
        Ok(Self {
            user_agent: user_agent.into(),
        })
    }
}

impl Default for HttpRegisterFlowTransport {
    fn default() -> Self {
        Self {
            user_agent: core_user_agent(),
        }
    }
}

async fn probe_client(homeserver_base_url: &str, user_agent: &str) -> Result<Client, AuthError> {
    let base = normalize_homeserver_url(homeserver_base_url)?.into_string();
    build_auth_probe_client(
        &base,
        user_agent,
        Duration::from_secs(AUTH_HTTP_TIMEOUT_SECS),
    )
    .await
    .map_err(|_| AuthError::Connectivity {
        diagnostic_id: "r0.7-http-client-init",
    })
}

impl RegisterFlowsTransport for HttpRegisterFlowTransport {
    async fn probe_register_flows(
        &self,
        homeserver_base_url: &str,
    ) -> Result<RegisterFlowsProbe, AuthError> {
        let client = probe_client(homeserver_base_url, &self.user_agent).await?;
        // `Client::send` rather than `MatrixAuth::register`: a server that
        // registers without UIA must not leave a session on this client.
        let error = match client.send(register::v3::Request::new()).await {
            Ok(_) => return Ok(RegisterFlowsProbe::InvalidRequest),
            Err(error) => error,
        };
        if let Some(info) = error.as_uiaa_response() {
            let body =
                serde_json::to_string(info).map_err(|_| AuthError::UnsupportedCapability {
                    diagnostic_id: "p2-register-flows-uiaa-response-invalid",
                })?;
            return parse_register_uiaa_json(&body);
        }
        match probe_failure(&error) {
            ProbeFailure::Status(401) => Err(AuthError::UnsupportedCapability {
                diagnostic_id: "p2-register-flows-uiaa-response-invalid",
            }),
            ProbeFailure::Status(status) => map_register_flows_status(status),
            ProbeFailure::Redirected => map_register_flows_status(302),
            // A success answer the SDK could not read: the server did not
            // challenge the empty request, as for any 2xx before.
            ProbeFailure::UnreadableSuccess => Ok(RegisterFlowsProbe::InvalidRequest),
            ProbeFailure::Transport(error) => Err(error),
        }
    }
}

impl LoginFlowTransport for HttpLoginFlowTransport {
    async fn fetch_login_flows(
        &self,
        homeserver_base_url: &str,
    ) -> Result<Vec<LoginFlow>, AuthError> {
        let client = probe_client(homeserver_base_url, &self.user_agent).await?;
        let response = client
            .matrix_auth()
            .get_login_types()
            .await
            .map_err(|error| match probe_failure(&error) {
                ProbeFailure::Status(status) => map_http_status(status),
                ProbeFailure::Redirected => map_http_status(302),
                ProbeFailure::UnreadableSuccess => AuthError::UnsupportedCapability {
                    diagnostic_id: "r0.7-login-types-json",
                },
                ProbeFailure::Transport(error) => error,
            })?;
        Ok(login_flows_from_sdk(&response.flows))
    }
}

/// SDK login types as Synara login flows, in server order.
pub(super) fn login_flows_from_sdk(flows: &[LoginType]) -> Vec<LoginFlow> {
    flows
        .iter()
        .map(|flow| match flow {
            LoginType::Token(token) => LoginFlow::token(token.get_login_token),
            other => LoginFlow::from_matrix_parts(other.login_type(), None),
        })
        .collect()
}

/// What a failed probe request got back. No URL, header or body survives.
enum ProbeFailure {
    /// The server answered with an error status (4xx/5xx).
    Status(u16),
    /// The server answered with a redirect, which the probe client refuses.
    Redirected,
    /// The server answered with success but a body the SDK could not read:
    /// ruma parses every answer below 400 as a success response.
    UnreadableSuccess,
    /// No answer: the connection failed or timed out.
    Transport(AuthError),
}

fn probe_failure(error: &HttpError) -> ProbeFailure {
    if error.as_uiaa_response().is_some() {
        return ProbeFailure::Status(401);
    }
    if let Some(api) = error.as_client_api_error() {
        return ProbeFailure::Status(api.status_code.as_u16());
    }
    match error {
        HttpError::Cached(inner) => probe_failure(inner),
        HttpError::Api(_) => ProbeFailure::UnreadableSuccess,
        HttpError::Reqwest(error) if error.is_redirect() => ProbeFailure::Redirected,
        HttpError::Reqwest(error) if error.is_timeout() || error.is_connect() => {
            ProbeFailure::Transport(AuthError::Connectivity {
                diagnostic_id: "r0.7-http-connect",
            })
        }
        HttpError::Reqwest(error) if error.is_request() => {
            ProbeFailure::Transport(AuthError::Connectivity {
                diagnostic_id: "r0.7-http-request",
            })
        }
        HttpError::Reqwest(_) => ProbeFailure::Transport(AuthError::HomeserverUnavailable {
            diagnostic_id: "r0.7-http-unavailable",
        }),
        _ => ProbeFailure::Transport(AuthError::Unknown {
            diagnostic_id: "r0.7-http-status",
        }),
    }
}

/// Parse `GET /login` flows JSON into Synara domain login flows.
pub fn parse_login_types_json(raw: &str) -> Result<Vec<LoginFlow>, AuthError> {
    let value: Value = serde_json::from_str(raw).map_err(|_| AuthError::UnsupportedCapability {
        diagnostic_id: "r0.7-login-types-json",
    })?;
    let flows =
        value
            .get("flows")
            .and_then(Value::as_array)
            .ok_or(AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-missing-flows",
            })?;
    let mut out = Vec::with_capacity(flows.len());
    for flow in flows {
        let matrix_type =
            flow.get("type")
                .and_then(Value::as_str)
                .ok_or(AuthError::UnsupportedCapability {
                    diagnostic_id: "r0.7-login-types-missing-type",
                })?;
        let get_login_token = flow.get("get_login_token").and_then(Value::as_bool);
        out.push(LoginFlow::from_matrix_parts(matrix_type, get_login_token));
    }
    Ok(out)
}

/// Classify an HTTP status without retaining the endpoint or response body.
fn map_http_status(status: u16) -> AuthError {
    match status {
        404 => AuthError::UnsupportedCapability {
            diagnostic_id: "r0.7-login-types-404",
        },
        408 | 429 | 502 | 503 | 504 => AuthError::Connectivity {
            diagnostic_id: "r0.7-http-retryable",
        },
        500..=599 => AuthError::HomeserverUnavailable {
            diagnostic_id: "r0.7-http-5xx",
        },
        400..=499 => AuthError::UnsupportedCapability {
            diagnostic_id: "r0.7-http-4xx",
        },
        _ => AuthError::Unknown {
            diagnostic_id: "r0.7-http-status",
        },
    }
}

/// Map only the empty registration-probe statuses that are safe to expose as
/// existing probe outcomes. Other statuses remain privacy-safe transport
/// errors; no response body is parsed outside a `401` UIAA challenge.
fn map_register_flows_status(status: u16) -> Result<RegisterFlowsProbe, AuthError> {
    match status {
        403 => Ok(RegisterFlowsProbe::RegistrationDisabled),
        408 => Err(AuthError::Connectivity {
            diagnostic_id: "p2-register-flows-http-retryable",
        }),
        429 => Ok(RegisterFlowsProbe::RateLimited),
        400..=499 => Ok(RegisterFlowsProbe::InvalidRequest),
        502..=504 => Err(AuthError::Connectivity {
            diagnostic_id: "p2-register-flows-http-retryable",
        }),
        500..=599 => Err(AuthError::HomeserverUnavailable {
            diagnostic_id: "p2-register-flows-http-5xx",
        }),
        _ => Err(AuthError::Unknown {
            diagnostic_id: "p2-register-flows-http-status",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::LoginFlowKind;

    #[test]
    fn parse_login_types_password_token_and_unknown() {
        let flows = parse_login_types_json(
            r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.token","get_login_token":true},{"type":"m.login.custom"}]}"#,
        )
        .unwrap();
        assert_eq!(flows.len(), 3);
        assert_eq!(flows[0].kind, LoginFlowKind::Password);
        assert_eq!(flows[1].get_login_token, Some(true));
        assert_eq!(flows[2].kind, LoginFlowKind::Unknown);
    }

    #[test]
    fn parse_login_types_rejects_bad_json() {
        assert!(matches!(
            parse_login_types_json("not-json"),
            Err(AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-json"
            })
        ));
    }

    #[test]
    fn map_http_status_privacy_safe() {
        assert!(matches!(
            map_http_status(404),
            AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-404"
            }
        ));
        assert!(matches!(
            map_http_status(503),
            AuthError::Connectivity { .. }
        ));
        assert!(matches!(
            map_http_status(500),
            AuthError::HomeserverUnavailable { .. }
        ));
    }

    #[test]
    fn transport_constructor_does_not_panic() {
        let _ = HttpLoginFlowTransport::new();
    }

    // R0.7 slice 2: real HTTP against a credential-free loopback CS stub.
    async fn serve_loopback_json_once(listener: &tokio::net::TcpListener, status: u16, body: &str) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = listener
            .accept()
            .await
            .expect("loopback accept login-types request");
        let mut buffer = vec![0_u8; 8192];
        let _ = socket.read(&mut buffer).await.expect("read request");
        let reason = match status {
            200 => "OK",
            404 => "Not Found",
            503 => "Service Unavailable",
            500 => "Internal Server Error",
            _ => "Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    }

    fn is_loopback_homeserver_url(raw: &str) -> bool {
        let Ok(url) = url::Url::parse(raw) else {
            return false;
        };
        url.scheme() == "http"
            && matches!(
                url.host_str(),
                Some("127.0.0.1") | Some("localhost") | Some("::1")
            )
            && url.username().is_empty()
            && url.password().is_none()
            && (url.path().is_empty() || url.path() == "/")
            && url.query().is_none()
            && url.fragment().is_none()
    }

    #[tokio::test]
    async fn live_login_types_against_loopback_cs_stub() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback CS stub");
        let address = listener.local_addr().expect("local address");
        let base = format!("http://{address}");
        assert!(is_loopback_homeserver_url(&base));

        let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.dummy"}]}"#;
        let server = tokio::spawn(async move {
            serve_loopback_json_once(&listener, 200, body).await;
        });

        let transport = HttpLoginFlowTransport::new().expect("HTTP transport");
        let result = crate::app::auth::discover_login_flows(&base, &transport)
            .await
            .expect("login-types from loopback CS stub");

        assert_eq!(result.homeserver_base_url, base.trim_end_matches('/'));
        assert!(result.password_available());
        assert!(result
            .flows
            .iter()
            .any(|flow| flow.matrix_type == "m.login.dummy"));
        server.await.expect("stub task");
    }

    #[tokio::test]
    async fn live_login_types_maps_stub_5xx_privately() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback CS stub");
        let address = listener.local_addr().expect("local address");
        let base = format!("http://{address}");
        let server = tokio::spawn(async move {
            serve_loopback_json_once(&listener, 503, r#"{"error":"upstream"}"#).await;
        });

        let transport = HttpLoginFlowTransport::new().expect("HTTP transport");
        let error = transport
            .fetch_login_flows(&base)
            .await
            .expect_err("503 must not parse as flows");
        let message = error.to_string();
        assert!(!message.contains("upstream"));
        assert!(!message.contains(&address.to_string()));
        assert!(matches!(
            error,
            AuthError::Connectivity {
                diagnostic_id: "r0.7-http-retryable"
            }
        ));
        server.await.expect("stub task");
    }

    #[test]
    fn registration_probe_statuses_expose_only_existing_safe_outcomes() {
        assert_eq!(
            map_register_flows_status(403).unwrap(),
            RegisterFlowsProbe::RegistrationDisabled
        );
        assert_eq!(
            map_register_flows_status(429).unwrap(),
            RegisterFlowsProbe::RateLimited
        );
        assert_eq!(
            map_register_flows_status(400).unwrap(),
            RegisterFlowsProbe::InvalidRequest
        );
        assert!(matches!(
            map_register_flows_status(503),
            Err(AuthError::Connectivity {
                diagnostic_id: "p2-register-flows-http-retryable"
            })
        ));
    }

    #[tokio::test]
    async fn registration_probe_redirect_is_not_followed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind registration redirect server");
        let address = listener
            .local_addr()
            .expect("registration redirect address");
        let base = format!("http://{address}");
        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let (mut socket, _) = listener.accept().await.expect("accept probe request");
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await.expect("read probe request");
            socket
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/redirect-target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("write redirect response");
        });

        let transport = HttpRegisterFlowTransport::new().expect("HTTP transport");
        let error = transport
            .probe_register_flows(&base)
            .await
            .expect_err("registration redirect must fail closed rather than be followed");
        assert!(matches!(
            error,
            AuthError::Unknown {
                diagnostic_id: "p2-register-flows-http-status"
            }
        ));
        server.await.expect("redirect server task");
    }

    #[tokio::test]
    async fn live_login_types_against_disposable_synapse_when_configured() {
        if std::env::var("SYNARA_RUN_MATRIX_RUST_AUTH_LIVE")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }
        let base = match std::env::var("SYNARA_MATRIX_HOMESERVER_URL") {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(
            is_loopback_homeserver_url(&base),
            "live auth transport tests accept only credential-free HTTP loopback homeserver URLs"
        );

        let transport = HttpLoginFlowTransport::new().expect("HTTP transport");
        let result = crate::app::auth::discover_login_flows(&base, &transport)
            .await
            .expect("disposable Synapse login-types listing");
        assert!(
            result.password_available(),
            "disposable Synapse must advertise m.login.password for harness registration/login paths"
        );
    }

    #[tokio::test]
    async fn login_types_redirect_is_not_followed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind redirect server");
        let address = listener.local_addr().expect("redirect address");
        let base = format!("http://{address}");
        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let (mut socket, _) = listener.accept().await.expect("accept redirect request");
            let mut request = [0_u8; 1024];
            let _ = socket
                .read(&mut request)
                .await
                .expect("read redirect request");
            socket
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/redirect-target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("write redirect response");
        });

        let transport = HttpLoginFlowTransport::new().expect("HTTP transport");
        let error = transport
            .fetch_login_flows(&base)
            .await
            .expect_err("redirect must fail closed rather than be followed");
        assert!(matches!(
            error,
            AuthError::Unknown {
                diagnostic_id: "r0.7-http-status"
            }
        ));
        server.await.expect("redirect server task");
    }

    #[tokio::test]
    async fn registration_probe_reads_the_sdk_uia_challenge_with_one_request() {
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        let server = MatrixMockServer::new().await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/register"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "session": "uia-session",
                "flows": [{"stages": ["m.login.registration_token", "m.login.dummy"]}],
                "params": {}
            })))
            .expect(1)
            .mount(server.server())
            .await;

        let transport =
            HttpRegisterFlowTransport::new_with_user_agent("Synara-iOS/2.2.6 (matrix-sdk/x)")
                .unwrap();
        let probe = transport
            .probe_register_flows(&server.uri())
            .await
            .expect("UIA challenge");

        let RegisterFlowsProbe::FlowRequired { session, flows, .. } = probe else {
            panic!("expected a UIA challenge, got {probe:?}");
        };
        assert_eq!(session.as_deref(), Some("uia-session"));
        assert_eq!(
            flows[0].stages,
            ["m.login.registration_token", "m.login.dummy"]
        );
        let requests = server.server().received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "no /versions or well-known request");
        assert_eq!(
            requests[0].headers.get("user-agent").unwrap(),
            "Synara-iOS/2.2.6 (matrix-sdk/x)"
        );
        assert!(!requests[0].headers.contains_key("authorization"));
    }

    #[tokio::test]
    async fn registration_probe_maps_disabled_registration() {
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        let server = MatrixMockServer::new().await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/register"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "errcode": "M_FORBIDDEN",
                "error": "Registration has been disabled"
            })))
            .mount(server.server())
            .await;

        let probe = HttpRegisterFlowTransport::new()
            .unwrap()
            .probe_register_flows(&server.uri())
            .await
            .unwrap();
        assert_eq!(probe, RegisterFlowsProbe::RegistrationDisabled);
    }

    #[tokio::test]
    async fn login_types_go_through_the_sdk_with_the_shell_user_agent() {
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        let server = MatrixMockServer::new().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "flows": [
                    {"type": "m.login.password"},
                    {"type": "m.login.token", "get_login_token": true}
                ]
            })))
            .expect(1)
            .mount(server.server())
            .await;

        let flows = HttpLoginFlowTransport::new_with_user_agent("Synara-Desktop/1.4.0")
            .unwrap()
            .fetch_login_flows(&server.uri())
            .await
            .unwrap();

        assert_eq!(flows, [LoginFlow::password(), LoginFlow::token(true)]);
        let requests = server.server().received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "no /versions or well-known request");
        assert_eq!(
            requests[0].headers.get("user-agent").unwrap(),
            "Synara-Desktop/1.4.0"
        );
    }
}
