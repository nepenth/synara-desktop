//! Homeserver well-known discovery service (P3.1).
//!
//! Network I/O is isolated behind [`DiscoveryTransport`] so unit tests can use
//! [`MockDiscoveryTransport`] without live homeservers or the Matrix SDK.
//!
//! **No** login, token handling, session restore, or production Tauri commands.

use super::error::AuthError;
use super::input::{
    normalize_homeserver_url, normalize_server_name, DiscoveryInput, DiscoveryInputKind,
    NormalizedHomeserverUrl,
};

/// Parsed `/.well-known/matrix/client` fields used by Synara (domain types only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WellKnownClientConfig {
    /// `m.homeserver.base_url` (required by the Matrix spec).
    pub homeserver_base_url: String,
    /// `m.identity_server.base_url` when present.
    pub identity_server_base_url: Option<String>,
}

impl WellKnownClientConfig {
    pub fn new(
        homeserver_base_url: impl Into<String>,
        identity_server_base_url: Option<String>,
    ) -> Result<Self, AuthError> {
        let hs = normalize_homeserver_url(&homeserver_base_url.into())?;
        let identity = match identity_server_base_url {
            Some(raw) => Some(normalize_homeserver_url(&raw)?.into_string()),
            None => None,
        };
        Ok(Self {
            homeserver_base_url: hs.into_string(),
            identity_server_base_url: identity,
        })
    }
}

/// Parse `/.well-known/matrix/client` JSON into a domain config (no secrets).
pub fn parse_well_known_client_json(raw: &str) -> Result<WellKnownClientConfig, AuthError> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| AuthError::UnsupportedCapability {
            diagnostic_id: "r0.7-well-known-json",
        })?;
    let hs = value
        .get("m.homeserver")
        .and_then(|object| object.get("base_url"))
        .and_then(serde_json::Value::as_str)
        .ok_or(AuthError::UnsupportedCapability {
            diagnostic_id: "r0.7-well-known-missing-homeserver",
        })?;
    let identity = value
        .get("m.identity_server")
        .and_then(|object| object.get("base_url"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    WellKnownClientConfig::new(hs, identity)
}

/// Successful homeserver resolution ready for client construction / login-flow lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryResult {
    pub input_kind: DiscoveryInputKind,
    /// Server name when known (from input or inferred). Not a URL.
    pub server_name: Option<String>,
    /// Resolved homeserver base URL for CS API (no trailing slash).
    pub homeserver_base_url: String,
    /// Optional identity server from well-known.
    pub identity_server_base_url: Option<String>,
    /// True when well-known was fetched and applied.
    pub used_well_known: bool,
}

impl DiscoveryResult {
    /// Privacy-safe diagnostic projection (URLs are product-visible homeserver
    /// locations, not secrets — still never include tokens).
    pub fn homeserver_url(&self) -> &str {
        &self.homeserver_base_url
    }

    /// Convert the resolved homeserver into a validated URL handle.
    pub fn normalized_homeserver(&self) -> Result<NormalizedHomeserverUrl, AuthError> {
        normalize_homeserver_url(&self.homeserver_base_url)
    }
}

/// Pluggable transport for `GET /.well-known/matrix/client` against a server name.
///
/// Production adapters (later tasks) may use the Matrix Rust SDK client builder
/// discovery path. P3.1 only requires the trait + mock for harness tests.
pub trait DiscoveryTransport {
    fn fetch_well_known(
        &self,
        server_name: &str,
    ) -> impl std::future::Future<Output = Result<WellKnownClientConfig, AuthError>> + Send;
}

/// In-memory mock transport for unit tests / harnesses.
#[derive(Debug, Clone, Default)]
pub struct MockDiscoveryTransport {
    /// Map of server_name → well-known result (or error via `errors`).
    pub responses: std::collections::HashMap<String, WellKnownClientConfig>,
    /// Map of server_name → forced error.
    pub errors: std::collections::HashMap<String, AuthError>,
    /// Default error when neither responses nor errors match.
    pub default_error: Option<AuthError>,
}

impl MockDiscoveryTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_response(
        mut self,
        server_name: impl Into<String>,
        config: WellKnownClientConfig,
    ) -> Self {
        self.responses.insert(server_name.into(), config);
        self
    }

    pub fn with_error(mut self, server_name: impl Into<String>, error: AuthError) -> Self {
        self.errors.insert(server_name.into(), error);
        self
    }

    pub fn with_default_error(mut self, error: AuthError) -> Self {
        self.default_error = Some(error);
        self
    }
}

impl DiscoveryTransport for MockDiscoveryTransport {
    async fn fetch_well_known(
        &self,
        server_name: &str,
    ) -> Result<WellKnownClientConfig, AuthError> {
        if let Some(err) = self.errors.get(server_name) {
            return Err(err.clone());
        }
        if let Some(cfg) = self.responses.get(server_name) {
            return Ok(cfg.clone());
        }
        Err(self
            .default_error
            .clone()
            .unwrap_or(AuthError::HomeserverUnavailable {
                diagnostic_id: "p3.1-mock-well-known-miss",
            }))
    }
}

/// Resolve a homeserver from validated input using the given transport.
///
/// - [`DiscoveryInput::HomeserverUrl`]: no network; normalize only
/// - [`DiscoveryInput::ServerName`]: well-known via transport
/// - [`DiscoveryInput::ServerNameOrUrl`]: try server-name well-known first; on
///   well-known failure that is connectivity/unavailable, fall back to treating
///   the input as an explicit homeserver URL when it has a scheme; otherwise
///   fail with the discovery error
pub async fn discover_homeserver<T: DiscoveryTransport>(
    input: &DiscoveryInput,
    transport: &T,
) -> Result<DiscoveryResult, AuthError> {
    match input {
        DiscoveryInput::HomeserverUrl(raw) => {
            let url = normalize_homeserver_url(raw)?;
            Ok(DiscoveryResult {
                input_kind: DiscoveryInputKind::HomeserverUrl,
                server_name: None,
                homeserver_base_url: url.into_string(),
                identity_server_base_url: None,
                used_well_known: false,
            })
        }
        DiscoveryInput::ServerName(raw) => {
            let name = normalize_server_name(raw)?;
            match transport.fetch_well_known(name.as_str()).await {
                Ok(wk) => Ok(DiscoveryResult {
                    input_kind: DiscoveryInputKind::ServerName,
                    server_name: Some(name.into_string()),
                    homeserver_base_url: wk.homeserver_base_url,
                    identity_server_base_url: wk.identity_server_base_url,
                    used_well_known: true,
                }),
                // Product autoDiscovery IGNORE (404): use https://{server} as base.
                Err(e) if e.allows_well_known_ignore_fallback() => {
                    Ok(fallback_https_base_for_server_name(
                        name.as_str(),
                        DiscoveryInputKind::ServerName,
                    )?)
                }
                Err(e) => Err(e),
            }
        }
        DiscoveryInput::ServerNameOrUrl(raw) => discover_server_name_or_url(raw, transport).await,
    }
}

/// Product-aligned IGNORE fallback: `https://{server_name}` as homeserver base.
fn fallback_https_base_for_server_name(
    server_name: &str,
    input_kind: DiscoveryInputKind,
) -> Result<DiscoveryResult, AuthError> {
    let base = format!("https://{server_name}");
    let url = normalize_homeserver_url(&base)?;
    Ok(DiscoveryResult {
        input_kind,
        server_name: Some(server_name.to_owned()),
        homeserver_base_url: url.into_string(),
        identity_server_base_url: None,
        used_well_known: false,
    })
}

async fn discover_server_name_or_url<T: DiscoveryTransport>(
    raw: &str,
    transport: &T,
) -> Result<DiscoveryResult, AuthError> {
    let trimmed = raw.trim();
    // Prefer well-known when the string is a plausible server name.
    if let Ok(name) = normalize_server_name(trimmed) {
        match transport.fetch_well_known(name.as_str()).await {
            Ok(wk) => {
                return Ok(DiscoveryResult {
                    input_kind: DiscoveryInputKind::ServerNameOrUrlAsServerName,
                    server_name: Some(name.into_string()),
                    homeserver_base_url: wk.homeserver_base_url,
                    identity_server_base_url: wk.identity_server_base_url,
                    used_well_known: true,
                });
            }
            Err(e) => {
                // Explicit URL input with scheme: fall back to that URL on any
                // well-known failure (matches product host→well-known→URL path).
                let lower = trimmed.to_ascii_lowercase();
                if lower.starts_with("http://") || lower.starts_with("https://") {
                    let url = normalize_homeserver_url(trimmed)?;
                    return Ok(DiscoveryResult {
                        input_kind: DiscoveryInputKind::ServerNameOrUrlAsUrl,
                        server_name: None,
                        homeserver_base_url: url.into_string(),
                        identity_server_base_url: None,
                        used_well_known: false,
                    });
                }
                // Bare server name + well-known 404 IGNORE → https://{name}.
                if e.allows_well_known_ignore_fallback() {
                    return fallback_https_base_for_server_name(
                        name.as_str(),
                        DiscoveryInputKind::ServerNameOrUrlAsServerName,
                    );
                }
                // Connectivity / hard unavailability: surface (no silent fallback).
                return Err(e);
            }
        }
    }

    // Not a server name — try as explicit URL.
    let url = normalize_homeserver_url(trimmed)?;
    Ok(DiscoveryResult {
        input_kind: DiscoveryInputKind::ServerNameOrUrlAsUrl,
        server_name: None,
        homeserver_base_url: url.into_string(),
        identity_server_base_url: None,
        used_well_known: false,
    })
}

/// Combined discovery + login-flow listing (mockable transports).
///
/// Orchestrates FR-7.1-001 then FR-7.1-005 for harness tests. No login.
pub async fn discover_homeserver_and_login_flows<D, L>(
    input: &DiscoveryInput,
    discovery: &D,
    login_flows: &L,
) -> Result<(DiscoveryResult, super::LoginFlowDiscoveryResult), AuthError>
where
    D: DiscoveryTransport,
    L: super::LoginFlowTransport,
{
    let discovered = discover_homeserver(input, discovery).await?;
    let flows = super::discover_login_flows(discovered.homeserver_url(), login_flows).await?;
    Ok((discovered, flows))
}
