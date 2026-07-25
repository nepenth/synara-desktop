//! Login-flow discovery service (P3.1).
//!
//! Given a resolved homeserver base URL, list available login mechanisms as
//! stable Synara domain types. Network I/O is behind [`LoginFlowTransport`].
//!
//! **No** password/token/SSO login execution (P3.2 / P3.3).

use super::error::AuthError;
use super::input::normalize_homeserver_url;

/// Stable Synara login-flow kinds (not raw SDK / Ruma enums).
///
/// Wire / IPC consumers should use these discriminators rather than
/// `m.login.*` strings alone; the Matrix type string is still available via
/// [`LoginFlowKind::matrix_type`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoginFlowKind {
    /// `m.login.password`
    Password,
    /// `m.login.token`
    Token,
    /// `m.login.sso`
    Sso,
    /// `m.login.application_service`
    ApplicationService,
    /// Unrecognized homeserver login type (type string preserved on [`LoginFlow`]).
    Unknown,
}

impl LoginFlowKind {
    pub const ALL_KNOWN: &'static [LoginFlowKind] = &[
        Self::Password,
        Self::Token,
        Self::Sso,
        Self::ApplicationService,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Token => "token",
            Self::Sso => "sso",
            Self::ApplicationService => "application_service",
            Self::Unknown => "unknown",
        }
    }

    /// Spec login type string for known kinds.
    pub fn matrix_type(self) -> Option<&'static str> {
        match self {
            Self::Password => Some("m.login.password"),
            Self::Token => Some("m.login.token"),
            Self::Sso => Some("m.login.sso"),
            Self::ApplicationService => Some("m.login.application_service"),
            Self::Unknown => None,
        }
    }

    /// Map a Matrix login type string to a Synara kind.
    pub fn from_matrix_type(matrix_type: &str) -> Self {
        match matrix_type {
            "m.login.password" => Self::Password,
            "m.login.token" => Self::Token,
            "m.login.sso" => Self::Sso,
            "m.login.application_service" => Self::ApplicationService,
            _ => Self::Unknown,
        }
    }
}

/// One SSO identity provider advertised by `m.login.sso`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsoIdentityProvider {
    pub id: String,
    pub name: String,
    /// Optional brand identifier (e.g. `github`, `google`) when provided.
    pub brand: Option<String>,
}

/// One homeserver login flow as a Synara domain value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFlow {
    pub kind: LoginFlowKind,
    /// Original Matrix type string (`m.login.password`, custom types, …).
    pub matrix_type: String,
    /// SSO IdPs when `kind == Sso`.
    pub identity_providers: Vec<SsoIdentityProvider>,
    /// Token flow: homeserver supports `get_login_token` (when known).
    pub get_login_token: Option<bool>,
    /// SSO flow preferred for OAuth-aware clients (when known).
    pub oauth_aware_preferred: Option<bool>,
}

impl LoginFlow {
    pub fn password() -> Self {
        Self {
            kind: LoginFlowKind::Password,
            matrix_type: "m.login.password".to_owned(),
            identity_providers: Vec::new(),
            get_login_token: None,
            oauth_aware_preferred: None,
        }
    }

    pub fn token(get_login_token: bool) -> Self {
        Self {
            kind: LoginFlowKind::Token,
            matrix_type: "m.login.token".to_owned(),
            identity_providers: Vec::new(),
            get_login_token: Some(get_login_token),
            oauth_aware_preferred: None,
        }
    }

    pub fn sso(
        identity_providers: Vec<SsoIdentityProvider>,
        oauth_aware_preferred: bool,
    ) -> Self {
        Self {
            kind: LoginFlowKind::Sso,
            matrix_type: "m.login.sso".to_owned(),
            identity_providers,
            get_login_token: None,
            oauth_aware_preferred: Some(oauth_aware_preferred),
        }
    }

    pub fn application_service() -> Self {
        Self {
            kind: LoginFlowKind::ApplicationService,
            matrix_type: "m.login.application_service".to_owned(),
            identity_providers: Vec::new(),
            get_login_token: None,
            oauth_aware_preferred: None,
        }
    }

    /// Map a raw Matrix type (+ optional SSO metadata) into a domain flow.
    pub fn from_matrix_parts(
        matrix_type: &str,
        identity_providers: Vec<SsoIdentityProvider>,
        get_login_token: Option<bool>,
        oauth_aware_preferred: Option<bool>,
    ) -> Self {
        let kind = LoginFlowKind::from_matrix_type(matrix_type);
        Self {
            kind,
            matrix_type: matrix_type.to_owned(),
            identity_providers,
            get_login_token,
            oauth_aware_preferred,
        }
    }
}

/// Result of listing login flows for one homeserver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFlowDiscoveryResult {
    pub homeserver_base_url: String,
    pub flows: Vec<LoginFlow>,
}

impl LoginFlowDiscoveryResult {
    pub fn supports(&self, kind: LoginFlowKind) -> bool {
        self.flows.iter().any(|f| f.kind == kind)
    }

    pub fn password_available(&self) -> bool {
        self.supports(LoginFlowKind::Password)
    }

    pub fn sso_available(&self) -> bool {
        self.supports(LoginFlowKind::Sso)
    }
}

/// Pluggable transport for the Matrix Client-Server login-types listing endpoint
/// (read-only; no credentials submitted).
///
/// Returns **Synara domain** flows — never raw SDK response types.
pub trait LoginFlowTransport {
    fn fetch_login_flows(
        &self,
        homeserver_base_url: &str,
    ) -> impl std::future::Future<Output = Result<Vec<LoginFlow>, AuthError>> + Send;
}

/// In-memory mock for unit tests / harnesses.
#[derive(Debug, Clone, Default)]
pub struct MockLoginFlowTransport {
    pub responses: std::collections::HashMap<String, Vec<LoginFlow>>,
    pub errors: std::collections::HashMap<String, AuthError>,
    pub default_error: Option<AuthError>,
}

impl MockLoginFlowTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_response(
        mut self,
        homeserver_base_url: impl Into<String>,
        flows: Vec<LoginFlow>,
    ) -> Self {
        // Normalize key for stable lookup.
        let key = normalize_homeserver_url(&homeserver_base_url.into())
            .map(|u| u.into_string())
            .unwrap_or_else(|_| String::new());
        self.responses.insert(key, flows);
        self
    }

    pub fn with_error(mut self, homeserver_base_url: impl Into<String>, error: AuthError) -> Self {
        let key = normalize_homeserver_url(&homeserver_base_url.into())
            .map(|u| u.into_string())
            .unwrap_or_else(|_| String::new());
        self.errors.insert(key, error);
        self
    }
}

impl LoginFlowTransport for MockLoginFlowTransport {
    async fn fetch_login_flows(
        &self,
        homeserver_base_url: &str,
    ) -> Result<Vec<LoginFlow>, AuthError> {
        let key = normalize_homeserver_url(homeserver_base_url)?.into_string();
        if let Some(err) = self.errors.get(&key) {
            return Err(err.clone());
        }
        if let Some(flows) = self.responses.get(&key) {
            return Ok(flows.clone());
        }
        Err(self.default_error.clone().unwrap_or(AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.1-mock-login-flows-miss",
        }))
    }
}

/// Discover login flows for a resolved homeserver base URL.
pub async fn discover_login_flows<T: LoginFlowTransport>(
    homeserver_base_url: &str,
    transport: &T,
) -> Result<LoginFlowDiscoveryResult, AuthError> {
    let url = normalize_homeserver_url(homeserver_base_url)?;
    let base = url.into_string();
    let flows = transport.fetch_login_flows(&base).await?;
    Ok(LoginFlowDiscoveryResult {
        homeserver_base_url: base,
        flows,
    })
}

/// Map fixture-style flow descriptors (tests / harness JSON) into domain flows.
///
/// Accepted matrix types: `m.login.password`, `m.login.token`, `m.login.sso`,
/// `m.login.application_service`, or any custom string → [`LoginFlowKind::Unknown`].
pub fn map_matrix_login_types(types: &[&str]) -> Vec<LoginFlow> {
    types
        .iter()
        .map(|t| LoginFlow::from_matrix_parts(t, Vec::new(), None, None))
        .collect()
}
