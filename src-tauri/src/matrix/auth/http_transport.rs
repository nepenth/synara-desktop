//! Desktop well-known HTTP adapter plus shared login-flow transport re-exports.
//!
//! Login-type HTTP parsing, well-known JSON parsing, and login-flow fetching
//! live in `synara-core::app::auth`; this shell file retains the live
//! well-known HTTP adapter (product user-agent).

use std::time::Duration;

use super::error::AuthError;
use crate::matrix::client_builder::default_user_agent;
use synara_core::app::auth::{DiscoveryTransport, WellKnownClientConfig};

pub use synara_core::app::auth::{
    parse_login_types_json, parse_well_known_client_json, HttpLoginFlowTransport,
    AUTH_HTTP_MAX_RESPONSE_BYTES, AUTH_HTTP_TIMEOUT_SECS,
};

/// Live HTTP transport for well-known discovery (CS API).
#[derive(Debug, Clone)]
pub struct HttpDiscoveryTransport {
    http: reqwest::Client,
}

impl HttpDiscoveryTransport {
    /// Build a transport with product user-agent and bounded timeout.
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

impl Default for HttpDiscoveryTransport {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
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
        status if (500..600).contains(&status) => AuthError::HomeserverUnavailable {
            diagnostic_id: "r0.7-http-5xx",
        },
        status if (400..500).contains(&status) => AuthError::UnsupportedCapability {
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
        assert!(matches!(
            parse_well_known_client_json(r#"{"m.identity_server":{"base_url":"https://x"}}"#),
            Err(AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-well-known-missing-homeserver"
            })
        ));
    }

    #[test]
    fn discovery_transport_constructor_does_not_panic() {
        let _ = HttpDiscoveryTransport::new();
    }
}
