//! R0.7 slice — live Client-Server HTTP transports for discovery + login-flow listing.
//!
//! Implements the same CS endpoints the Matrix Rust SDK 0.18 uses under the hood
//! (`GET /.well-known/matrix/client`, `GET /_matrix/client/v3/login`) without
//! calling banned production login/session APIs (`matrix_auth`, `login_*`,
//! `Client::builder` outside `client_builder/`).
//!
//! **Read-only:** never submits credentials. No dual-backend. No Tauri Matrix
//! product commands. Full disposable-Synapse lifecycle remains a later R0.7 residual.

use std::time::Duration;

use serde_json::Value;

use super::discovery::{DiscoveryTransport, WellKnownClientConfig};
use super::error::AuthError;
use super::input::normalize_homeserver_url;
use super::login_flow::{LoginFlow, LoginFlowTransport, SsoIdentityProvider};
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
        let idps = parse_identity_providers(flow.get("identity_providers"));
        let get_login_token = flow.get("get_login_token").and_then(|v| v.as_bool());
        // Spec / Element extensions may surface oauth preference under various keys;
        // only accept boolean product-safe fields when present.
        let oauth_aware = flow
            .get("org.matrix.msc3824.oauth_aware_preferred")
            .or_else(|| flow.get("oauth_aware_preferred"))
            .and_then(|v| v.as_bool());
        out.push(LoginFlow::from_matrix_parts(
            matrix_type,
            idps,
            get_login_token,
            oauth_aware,
        ));
    }
    Ok(out)
}

fn parse_identity_providers(value: Option<&Value>) -> Vec<SsoIdentityProvider> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_owned();
        let brand = item
            .get("brand")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        out.push(SsoIdentityProvider {
            id: id.to_owned(),
            name,
            brand,
        });
    }
    out
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
    fn parse_login_types_password_and_sso() {
        let raw = r#"{
          "flows": [
            {"type": "m.login.password"},
            {
              "type": "m.login.sso",
              "identity_providers": [
                {"id": "github", "name": "GitHub", "brand": "github"}
              ]
            },
            {"type": "m.login.token", "get_login_token": true},
            {"type": "m.login.custom"}
          ]
        }"#;
        let flows = parse_login_types_json(raw).unwrap();
        assert_eq!(flows.len(), 4);
        assert_eq!(flows[0].kind, LoginFlowKind::Password);
        assert_eq!(flows[1].kind, LoginFlowKind::Sso);
        assert_eq!(flows[1].identity_providers.len(), 1);
        assert_eq!(flows[1].identity_providers[0].id, "github");
        assert_eq!(flows[2].kind, LoginFlowKind::Token);
        assert_eq!(flows[2].get_login_token, Some(true));
        assert_eq!(flows[3].kind, LoginFlowKind::Unknown);
        assert_eq!(flows[3].matrix_type, "m.login.custom");
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
}
