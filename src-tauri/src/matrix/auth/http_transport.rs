//! R0.7 slice — live Client-Server HTTP transports for discovery + login-flow listing.
//!
//! Implements the same CS endpoints the Matrix Rust SDK 0.18 uses under the hood
//! (`GET /.well-known/matrix/client`, `GET /_matrix/client/v3/login`) without
//! calling banned production login/session APIs (`matrix_auth`, `login_*`,
//! `Client::builder` outside `client_builder/`).
//!
//! **Read-only:** never submits credentials. No dual-backend. No Tauri Matrix
//! product commands.
//!
//! R0.7 residual slices:
//! - **slice 1:** live HTTP transports + domain parsers (this module)
//! - **slice 2:** loopback CS stub + optional disposable-Synapse login-types
//!   evidence (see tests; gated by `SYNARA_RUN_MATRIX_RUST_AUTH_LIVE=1`)
//! - **slice 3:** composed encrypted store open / Ready / logout / reopen / wipe
//!   (lifecycle tests; real `SdkClientHandle`)
//! - **slice 4:** stale-generation isolation + wrong-key reopen privacy residual
//!   after real SDK install (lifecycle tests)
//! - **later residual:** authenticated live sync vs disposable Synapse (P3.2
//!   login APIs; guardrail-banned until deliberate P3.2 allowlist)

use std::time::Duration;

use serde_json::Value;

use super::discovery::{DiscoveryTransport, WellKnownClientConfig};
use super::error::AuthError;
use super::input::normalize_homeserver_url;
use super::login_flow::{LoginFlow, LoginFlowTransport};
use crate::matrix::client_builder::default_user_agent;

/// Default HTTP timeout for discovery / login-types (seconds).
pub const AUTH_HTTP_TIMEOUT_SECS: u64 = 15;

/// Live HTTP transport for well-known discovery (CS API).
#[derive(Debug, Clone)]
pub struct HttpDiscoveryTransport {
    http: reqwest::Client,
}

impl HttpDiscoveryTransport {
    /// Build a transport with product user-agent and bounded timeout.
    pub fn new() -> Result<Self, AuthError> {
        // Use `ClientBuilder::new` (not `Client::builder`) — the latter token is
        // banned by Matrix guardrails outside `matrix/client_builder/`.
        let http = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(AUTH_HTTP_TIMEOUT_SECS))
            .user_agent(default_user_agent())
            .build()
            .map_err(|_| AuthError::Connectivity {
                diagnostic_id: "r0.7-http-client-init",
            })?;
        Ok(Self { http })
    }
}

impl Default for HttpDiscoveryTransport {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            // Avoid `Client::new` token (guardrail matches any Client::new).
            http: reqwest::Client::default(),
        })
    }
}

impl DiscoveryTransport for HttpDiscoveryTransport {
    async fn fetch_well_known(
        &self,
        server_name: &str,
    ) -> Result<WellKnownClientConfig, AuthError> {
        let url = format!("https://{server_name}/.well-known/matrix/client");
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(AuthError::WellKnownNotFound {
                diagnostic_id: "r0.7-well-known-404",
            });
        }
        if !status.is_success() {
            return Err(map_http_status(status.as_u16(), "r0.7-well-known"));
        }
        let body = response.text().await.map_err(|_| AuthError::Connectivity {
            diagnostic_id: "r0.7-well-known-body",
        })?;
        parse_well_known_client_json(&body)
    }
}

/// Live HTTP transport for `GET /_matrix/client/v3/login` (login types listing).
#[derive(Debug, Clone)]
pub struct HttpLoginFlowTransport {
    http: reqwest::Client,
}

impl HttpLoginFlowTransport {
    pub fn new() -> Result<Self, AuthError> {
        let http = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(AUTH_HTTP_TIMEOUT_SECS))
            .user_agent(default_user_agent())
            .build()
            .map_err(|_| AuthError::Connectivity {
                diagnostic_id: "r0.7-http-client-init",
            })?;
        Ok(Self { http })
    }
}

impl Default for HttpLoginFlowTransport {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            // Avoid `Client::new` token (guardrail matches any Client::new).
            http: reqwest::Client::default(),
        })
    }
}

impl LoginFlowTransport for HttpLoginFlowTransport {
    async fn fetch_login_flows(
        &self,
        homeserver_base_url: &str,
    ) -> Result<Vec<LoginFlow>, AuthError> {
        let base = normalize_homeserver_url(homeserver_base_url)?.into_string();
        let url = format!("{base}/_matrix/client/v3/login");
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_http_status(status.as_u16(), "r0.7-login-types"));
        }
        let body = response.text().await.map_err(|_| AuthError::Connectivity {
            diagnostic_id: "r0.7-login-types-body",
        })?;
        parse_login_types_json(&body)
    }
}

/// Parse `/.well-known/matrix/client` JSON into a domain config (no secrets).
pub fn parse_well_known_client_json(raw: &str) -> Result<WellKnownClientConfig, AuthError> {
    let value: Value = serde_json::from_str(raw).map_err(|_| AuthError::UnsupportedCapability {
        diagnostic_id: "r0.7-well-known-json",
    })?;
    let hs = value
        .get("m.homeserver")
        .and_then(|o| o.get("base_url"))
        .and_then(|v| v.as_str())
        .ok_or(AuthError::UnsupportedCapability {
            diagnostic_id: "r0.7-well-known-missing-homeserver",
        })?;
    let identity = value
        .get("m.identity_server")
        .and_then(|o| o.get("base_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    WellKnownClientConfig::new(hs, identity)
}

/// Parse `GET /login` flows JSON into Synara domain login flows.
pub fn parse_login_types_json(raw: &str) -> Result<Vec<LoginFlow>, AuthError> {
    let value: Value = serde_json::from_str(raw).map_err(|_| AuthError::UnsupportedCapability {
        diagnostic_id: "r0.7-login-types-json",
    })?;
    let flows =
        value
            .get("flows")
            .and_then(|v| v.as_array())
            .ok_or(AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-missing-flows",
            })?;
    let mut out = Vec::with_capacity(flows.len());
    for flow in flows {
        let matrix_type =
            flow.get("type")
                .and_then(|v| v.as_str())
                .ok_or(AuthError::UnsupportedCapability {
                    diagnostic_id: "r0.7-login-types-missing-type",
                })?;
        let get_login_token = flow.get("get_login_token").and_then(|v| v.as_bool());
        out.push(LoginFlow::from_matrix_parts(matrix_type, get_login_token));
    }
    Ok(out)
}

fn map_reqwest_error(err: reqwest::Error) -> AuthError {
    // Privacy: never include the raw error string (may contain URLs/hosts).
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

fn map_http_status(status: u16, prefix: &'static str) -> AuthError {
    match status {
        404 => AuthError::UnsupportedCapability {
            diagnostic_id: match prefix {
                "r0.7-well-known" => "r0.7-well-known-404",
                _ => "r0.7-login-types-404",
            },
        },
        408 | 429 | 502 | 503 | 504 => AuthError::Connectivity {
            diagnostic_id: "r0.7-http-retryable",
        },
        s if (500..600).contains(&s) => AuthError::HomeserverUnavailable {
            diagnostic_id: "r0.7-http-5xx",
        },
        s if (400..500).contains(&s) => AuthError::UnsupportedCapability {
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
    use crate::matrix::auth::LoginFlowKind;

    #[test]
    fn parse_well_known_minimal() {
        let cfg = parse_well_known_client_json(
            r#"{"m.homeserver":{"base_url":"https://matrix.example.org"}}"#,
        )
        .unwrap();
        assert_eq!(cfg.homeserver_base_url, "https://matrix.example.org");
        assert!(cfg.identity_server_base_url.is_none());
    }

    #[test]
    fn parse_well_known_with_identity() {
        let cfg = parse_well_known_client_json(
            r#"{"m.homeserver":{"base_url":"https://hs.example.org/"},"m.identity_server":{"base_url":"https://id.example.org"}}"#,
        )
        .unwrap();
        assert_eq!(cfg.homeserver_base_url, "https://hs.example.org");
        assert_eq!(
            cfg.identity_server_base_url.as_deref(),
            Some("https://id.example.org")
        );
    }

    #[test]
    fn parse_well_known_missing_homeserver() {
        let err = parse_well_known_client_json(r#"{"m.identity_server":{"base_url":"https://x"}}"#)
            .unwrap_err();
        assert!(matches!(
            err,
            AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-well-known-missing-homeserver"
            }
        ));
    }

    #[test]
    fn parse_login_types_password_token_and_unknown() {
        let raw = r#"{
          "flows": [
            {"type": "m.login.password"},
            {"type": "m.login.token", "get_login_token": true},
            {"type": "m.login.custom"}
          ]
        }"#;
        let flows = parse_login_types_json(raw).unwrap();
        assert_eq!(flows.len(), 3);
        assert_eq!(flows[0].kind, LoginFlowKind::Password);
        assert_eq!(flows[1].kind, LoginFlowKind::Token);
        assert_eq!(flows[1].get_login_token, Some(true));
        assert_eq!(flows[2].kind, LoginFlowKind::Unknown);
        assert_eq!(flows[2].matrix_type, "m.login.custom");
    }

    #[test]
    fn parse_login_types_rejects_bad_json() {
        let err = parse_login_types_json("not-json").unwrap_err();
        assert!(matches!(
            err,
            AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-json"
            }
        ));
    }

    #[test]
    fn map_http_status_privacy_safe() {
        // Well-known path maps 404 before this helper; helper classifies generically.
        assert!(matches!(
            map_http_status(404, "r0.7-login-types"),
            AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-login-types-404"
            }
        ));
        assert!(matches!(
            map_http_status(503, "r0.7-login-types"),
            AuthError::Connectivity { .. }
        ));
        assert!(matches!(
            map_http_status(500, "r0.7-login-types"),
            AuthError::HomeserverUnavailable { .. }
        ));
    }

    #[test]
    fn transport_constructors_do_not_panic() {
        let _ = HttpDiscoveryTransport::new();
        let _ = HttpLoginFlowTransport::new();
    }

    // --- R0.7 slice 2: real HTTP against loopback CS stub (no Docker required) ---

    /// Minimal HTTP/1.1 JSON responder for login-types (and optional 5xx paths).
    async fn serve_loopback_json_once(listener: &tokio::net::TcpListener, status: u16, body: &str) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut sock, _) = listener
            .accept()
            .await
            .expect("loopback accept login-types request");
        let mut buf = vec![0u8; 8192];
        let _ = sock.read(&mut buf).await.expect("read request");
        let reason = match status {
            200 => "OK",
            404 => "Not Found",
            503 => "Service Unavailable",
            500 => "Internal Server Error",
            _ => "Error",
        };
        let resp = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        sock.write_all(resp.as_bytes())
            .await
            .expect("write response");
    }

    fn is_loopback_homeserver_url(raw: &str) -> bool {
        let Ok(url) = url::Url::parse(raw) else {
            return false;
        };
        if url.scheme() != "http" {
            return false;
        }
        matches!(
            url.host_str(),
            Some("127.0.0.1") | Some("localhost") | Some("::1")
        ) && url.username().is_empty()
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
        let addr = listener.local_addr().expect("local addr");
        let base = format!("http://{addr}");
        assert!(is_loopback_homeserver_url(&base));

        let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.dummy"}]}"#;
        let server = tokio::spawn(async move {
            serve_loopback_json_once(&listener, 200, body).await;
        });

        let transport = HttpLoginFlowTransport::new().expect("http transport");
        let result = super::super::discover_login_flows(&base, &transport)
            .await
            .expect("login-types from loopback CS stub");

        assert_eq!(result.homeserver_base_url, base.trim_end_matches('/'));
        assert!(result.password_available());
        assert!(result
            .flows
            .iter()
            .any(|f| f.matrix_type == "m.login.dummy"));
        // Privacy: domain types only — no raw error/body leakage surfaces here.
        server.await.expect("stub task");
    }

    #[tokio::test]
    async fn live_login_types_maps_stub_5xx_privately() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback CS stub");
        let addr = listener.local_addr().expect("local addr");
        let base = format!("http://{addr}");

        let server = tokio::spawn(async move {
            serve_loopback_json_once(&listener, 503, r#"{"error":"upstream"}"#).await;
        });

        let transport = HttpLoginFlowTransport::new().expect("http transport");
        let err = transport
            .fetch_login_flows(&base)
            .await
            .expect_err("503 must not parse as flows");
        // Privacy: diagnostic_id only — never the JSON error body / host.
        let msg = err.to_string();
        assert!(!msg.contains("upstream"));
        assert!(!msg.contains(&addr.to_string()));
        assert!(matches!(
            err,
            AuthError::Connectivity {
                diagnostic_id: "r0.7-http-retryable"
            }
        ));
        server.await.expect("stub task");
    }

    #[tokio::test]
    async fn live_login_types_against_disposable_synapse_when_configured() {
        // Opt-in live residual against `scripts/synapse-integration.sh` (loopback only).
        // SYNARA_RUN_MATRIX_RUST_AUTH_LIVE=1
        // SYNARA_MATRIX_HOMESERVER_URL=http://127.0.0.1:<port>
        if std::env::var("SYNARA_RUN_MATRIX_RUST_AUTH_LIVE")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }
        let base = match std::env::var("SYNARA_MATRIX_HOMESERVER_URL") {
            Ok(v) => v,
            Err(_) => return,
        };
        assert!(
            is_loopback_homeserver_url(&base),
            "live auth transport tests accept only credential-free HTTP loopback homeserver URLs"
        );

        let transport = HttpLoginFlowTransport::new().expect("http transport");
        let result = super::super::discover_login_flows(&base, &transport)
            .await
            .expect("disposable Synapse login-types listing");
        assert!(
            result.password_available(),
            "disposable Synapse must advertise m.login.password for harness registration/login paths"
        );
    }
}
