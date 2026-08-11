//! Bounded, redirect-free HTTP transport for Matrix login-flow discovery.
//!
//! It only sends `GET /_matrix/client/v3/login`; credentials and response
//! bodies never cross its public error surface.

use std::time::Duration;

use serde_json::Value;

use super::error::AuthError;
use super::input::normalize_homeserver_url;
use super::login_flow::{LoginFlow, LoginFlowTransport};

/// Default end-to-end timeout for the unauthenticated login-types request.
pub const AUTH_HTTP_TIMEOUT_SECS: u64 = 15;
/// Ceiling for a single login-types JSON response.
pub const AUTH_HTTP_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// Live read-only transport for `GET /_matrix/client/v3/login`.
#[derive(Debug, Clone)]
pub struct HttpLoginFlowTransport {
    http: reqwest::Client,
}

impl HttpLoginFlowTransport {
    /// Build the shared core default bounded client.
    pub fn new() -> Result<Self, AuthError> {
        Self::new_with_user_agent(concat!(
            "Synara-Core/",
            env!("CARGO_PKG_VERSION"),
            " (matrix-sdk/0.18.0)"
        ))
    }

    /// Build a bounded client with a shell-owned product identifier. This
    /// preserves desktop's established user agent without importing a shell
    /// type or product configuration into the core.
    pub fn new_with_user_agent(user_agent: impl Into<String>) -> Result<Self, AuthError> {
        let http = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(AUTH_HTTP_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(user_agent.into())
            .build()
            .map_err(|_| AuthError::Connectivity {
                diagnostic_id: "r0.7-http-client-init",
            })?;
        Ok(Self { http })
    }
}

impl Default for HttpLoginFlowTransport {
    fn default() -> Self {
        // Never fall back to an unconstrained client: that would silently
        // re-enable redirects or lose the request timeout. Construction must
        // fail closed if the HTTP backend cannot initialize.
        Self::new().expect("bounded login-flow HTTP client must initialize")
    }
}

impl LoginFlowTransport for HttpLoginFlowTransport {
    async fn fetch_login_flows(
        &self,
        homeserver_base_url: &str,
    ) -> Result<Vec<LoginFlow>, AuthError> {
        let base = normalize_homeserver_url(homeserver_base_url)?.into_string();
        let url = format!("{base}/_matrix/client/v3/login");
        let mut response = self.http.get(url).send().await.map_err(map_reqwest_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_http_status(status.as_u16()));
        }
        let body = read_bounded_response(&mut response).await?;
        parse_login_types_json(&body)
    }
}

async fn read_bounded_response(response: &mut reqwest::Response) -> Result<String, AuthError> {
    if response
        .content_length()
        .is_some_and(|length| length > AUTH_HTTP_MAX_RESPONSE_BYTES as u64)
    {
        return Err(AuthError::UnsupportedCapability {
            diagnostic_id: "r0.7-login-types-response-too-large",
        });
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AuthError::Connectivity {
            diagnostic_id: "r0.7-login-types-body",
        })?
    {
        if body.len().saturating_add(chunk.len()) > AUTH_HTTP_MAX_RESPONSE_BYTES {
            return Err(AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-response-too-large",
            });
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| AuthError::UnsupportedCapability {
        diagnostic_id: "r0.7-login-types-json",
    })
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

/// Classify an HTTP-library failure without exposing an endpoint or raw error.
fn map_reqwest_error(err: reqwest::Error) -> AuthError {
    if err.is_timeout() || err.is_connect() {
        return AuthError::Connectivity {
            diagnostic_id: "r0.7-http-connect",
        };
    }
    if err.is_request() {
        return AuthError::Connectivity {
            diagnostic_id: "r0.7-http-request",
        };
    }
    AuthError::HomeserverUnavailable {
        diagnostic_id: "r0.7-http-unavailable",
    }
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
    async fn login_types_response_body_is_bounded_before_reading() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind oversized server");
        let address = listener.local_addr().expect("oversized address");
        let base = format!("http://{address}");
        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let (mut socket, _) = listener.accept().await.expect("accept oversized request");
            let mut request = [0_u8; 1024];
            let _ = socket
                .read(&mut request)
                .await
                .expect("read oversized request");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{{}}",
                AUTH_HTTP_MAX_RESPONSE_BYTES + 1
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write oversized response");
        });

        let transport = HttpLoginFlowTransport::new().expect("HTTP transport");
        let error = transport
            .fetch_login_flows(&base)
            .await
            .expect_err("oversized response must fail closed");
        assert!(matches!(
            error,
            AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-response-too-large"
            }
        ));
        server.await.expect("oversized server task");
    }
}
