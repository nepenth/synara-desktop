//! Login-flow discovery service (P3.1).
//!
//! Given a resolved homeserver base URL, list available login mechanisms as
//! stable Synara domain types. Network I/O is behind [`LoginFlowTransport`].
//!
//! **No** password login execution (P3.2 / product owns that path).

use super::error::AuthError;
use super::input::normalize_homeserver_url;

use serde::{Deserialize, Serialize};

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
    /// `m.login.application_service`
    ApplicationService,
    /// Unrecognized homeserver login type (type string preserved on [`LoginFlow`]).
    Unknown,
}

impl LoginFlowKind {
    pub const ALL_KNOWN: &'static [LoginFlowKind] =
        &[Self::Password, Self::Token, Self::ApplicationService];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Token => "token",
            Self::ApplicationService => "application_service",
            Self::Unknown => "unknown",
        }
    }

    /// Spec login type string for known kinds.
    pub fn matrix_type(self) -> Option<&'static str> {
        match self {
            Self::Password => Some("m.login.password"),
            Self::Token => Some("m.login.token"),
            Self::ApplicationService => Some("m.login.application_service"),
            Self::Unknown => None,
        }
    }

    /// Map a Matrix login type string to a Synara kind.
    pub fn from_matrix_type(matrix_type: &str) -> Self {
        match matrix_type {
            "m.login.password" => Self::Password,
            "m.login.token" => Self::Token,
            "m.login.application_service" => Self::ApplicationService,
            _ => Self::Unknown,
        }
    }
}

/// One homeserver login flow as a Synara domain value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFlow {
    pub kind: LoginFlowKind,
    /// Original Matrix type string (`m.login.password`, custom types, …).
    pub matrix_type: String,
    /// Token flow: homeserver supports `get_login_token` (when known).
    pub get_login_token: Option<bool>,
}

impl LoginFlow {
    pub fn password() -> Self {
        Self {
            kind: LoginFlowKind::Password,
            matrix_type: "m.login.password".to_owned(),
            get_login_token: None,
        }
    }

    pub fn token(get_login_token: bool) -> Self {
        Self {
            kind: LoginFlowKind::Token,
            matrix_type: "m.login.token".to_owned(),
            get_login_token: Some(get_login_token),
        }
    }

    pub fn application_service() -> Self {
        Self {
            kind: LoginFlowKind::ApplicationService,
            matrix_type: "m.login.application_service".to_owned(),
            get_login_token: None,
        }
    }

    /// Map a raw Matrix type (+ optional token capability) into a domain flow.
    pub fn from_matrix_parts(matrix_type: &str, get_login_token: Option<bool>) -> Self {
        let kind = LoginFlowKind::from_matrix_type(matrix_type);
        Self {
            kind,
            matrix_type: matrix_type.to_owned(),
            get_login_token,
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
}

/// React/Tauri login-flow item. This is deliberately credential-free.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLoginFlowDto {
    /// Synara discriminator (`password`, `token`, `application_service`, `unknown`).
    pub kind: String,
    /// Original Matrix login type (`m.login.password`, custom types, …).
    pub matrix_type: String,
    /// Token flow capability when the homeserver supplied it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_login_token: Option<bool>,
}

impl From<LoginFlow> for MatrixLoginFlowDto {
    fn from(flow: LoginFlow) -> Self {
        Self {
            kind: flow.kind.as_str().to_owned(),
            matrix_type: flow.matrix_type,
            get_login_token: flow.get_login_token,
        }
    }
}

/// Exact successful React/Tauri response for `matrix_login_flows`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLoginFlowsResponse {
    pub flows: Vec<MatrixLoginFlowDto>,
}

/// Convert discovered domain flows to the stable command response.
pub fn login_flows_response(flows: Vec<LoginFlow>) -> MatrixLoginFlowsResponse {
    MatrixLoginFlowsResponse {
        flows: flows.into_iter().map(MatrixLoginFlowDto::from).collect(),
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
        Err(self
            .default_error
            .clone()
            .unwrap_or(AuthError::HomeserverUnavailable {
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
/// Accepted matrix types: `m.login.password`, `m.login.token`,
/// `m.login.application_service`, or any custom string → [`LoginFlowKind::Unknown`].
pub fn map_matrix_login_types(types: &[&str]) -> Vec<LoginFlow> {
    types
        .iter()
        .map(|t| LoginFlow::from_matrix_parts(t, None))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MatrixIpcErrorCategory;

    #[test]
    fn login_flow_list_mapping() {
        let mapped = map_matrix_login_types(&[
            "m.login.password",
            "m.login.token",
            "m.login.application_service",
            "m.login.custom.widget",
        ]);
        assert_eq!(mapped.len(), 4);
        assert_eq!(mapped[0].kind, LoginFlowKind::Password);
        assert_eq!(mapped[1].kind, LoginFlowKind::Token);
        assert_eq!(mapped[2].kind, LoginFlowKind::ApplicationService);
        assert_eq!(mapped[3].kind, LoginFlowKind::Unknown);
        assert_eq!(mapped[3].matrix_type, "m.login.custom.widget");
    }

    #[test]
    fn login_flow_discovery_mock_success() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let flows = vec![LoginFlow::password(), LoginFlow::token(true)];
        let transport =
            MockLoginFlowTransport::new().with_response("https://hs.example.org", flows.clone());
        let result = runtime
            .block_on(discover_login_flows("https://hs.example.org/", &transport))
            .expect("flows");
        assert_eq!(result.homeserver_base_url, "https://hs.example.org");
        assert!(result.password_available());
        assert!(result.supports(LoginFlowKind::Token));
        assert_eq!(result.flows.len(), 2);
        assert_eq!(result.flows[1].get_login_token, Some(true));
    }

    #[test]
    fn login_flow_discovery_mock_failure() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let transport = MockLoginFlowTransport::new().with_error(
            "https://hs.example.org",
            AuthError::Connectivity {
                diagnostic_id: "p3.1-login-flows-offline",
            },
        );
        let error = runtime
            .block_on(discover_login_flows("https://hs.example.org", &transport))
            .expect_err("offline");
        assert_eq!(error.category(), MatrixIpcErrorCategory::Connectivity);
    }

    #[test]
    fn login_flow_kind_matrix_type_roundtrip() {
        for kind in LoginFlowKind::ALL_KNOWN {
            let matrix_type = kind.matrix_type().expect("known type");
            assert_eq!(LoginFlowKind::from_matrix_type(matrix_type), *kind);
            assert!(!kind.as_str().is_empty());
        }
    }
}
