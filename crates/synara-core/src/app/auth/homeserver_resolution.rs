//! SDK-backed homeserver resolution for sign-in (review S4 / S7).
//!
//! Resolves what the user typed with the SDK on a memory-only client:
//! `.well-known/matrix/client` delegation for a server name, falling back to
//! `/versions` on the input as a homeserver URL. The resolved homeserver then
//! reports its login flows and whether native simplified sliding sync
//! (MSC4186) is available. No credential, session, store or vault is involved.

use std::time::Duration;

use matrix_sdk::ruma::api::client::session::get_login_types::v3::LoginType;
use matrix_sdk::Client;

use super::error::AuthError;
use super::http_transport::{core_user_agent, AUTH_HTTP_TIMEOUT_SECS};
use super::input::normalize_homeserver_url;
use super::login_flow::LoginFlow;
use crate::app::client_builder::{build_discovery_client, ClientBuilderError, TimeoutPolicy};
use crate::transport::MatrixIpcErrorCategory;

/// Bound on the whole resolution: well-known, `/versions` and `/login`.
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(30);
/// Longest input accepted from a sign-in field.
const MAX_INPUT_CHARS: usize = 2048;

/// A homeserver resolved for sign-in. URLs are product-visible locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHomeserver {
    /// Client-API base URL to sign in against (no trailing slash).
    pub homeserver_url: String,
    /// Login flows the resolved homeserver advertises.
    pub login_flows: Vec<LoginFlow>,
    /// The SDK would select native simplified sliding sync (MSC4186).
    pub native_sliding_sync: bool,
}

/// Resolve a typed server name or homeserver URL with the SDK.
pub async fn resolve_homeserver(server_name_or_url: &str) -> Result<ResolvedHomeserver, AuthError> {
    resolve_homeserver_with_user_agent(server_name_or_url, &core_user_agent()).await
}

/// [`resolve_homeserver`] with a shell-owned user agent.
pub async fn resolve_homeserver_with_user_agent(
    server_name_or_url: &str,
    user_agent: &str,
) -> Result<ResolvedHomeserver, AuthError> {
    let input = validate_input(server_name_or_url)?;
    tokio::time::timeout(RESOLVE_TIMEOUT, resolve(input, user_agent))
        .await
        .map_err(|_| AuthError::Connectivity {
            diagnostic_id: "p3.1-resolve-timeout",
        })?
}

async fn resolve(input: &str, user_agent: &str) -> Result<ResolvedHomeserver, AuthError> {
    let client = build_discovery_client(
        input,
        user_agent,
        TimeoutPolicy {
            request_timeout: Duration::from_secs(AUTH_HTTP_TIMEOUT_SECS),
            retry_limit: 1,
        },
    )
    .await
    .map_err(map_build_error)?;
    let homeserver_url = normalize_homeserver_url(client.homeserver().as_str())?.into_string();
    // A well-known target has not been checked yet; `/versions` validates it
    // and carries the sliding-sync verdict for both paths.
    let supported =
        client
            .supported_versions()
            .await
            .map_err(|_| AuthError::HomeserverUnavailable {
                diagnostic_id: "p3.1-resolve-versions-unavailable",
            })?;
    let native_sliding_sync = crate::app::sync::supports_native_sliding_sync(&supported);
    let login_flows = login_flows(&client).await?;
    Ok(ResolvedHomeserver {
        homeserver_url,
        login_flows,
        native_sliding_sync,
    })
}

async fn login_flows(client: &Client) -> Result<Vec<LoginFlow>, AuthError> {
    let response = client.matrix_auth().get_login_types().await.map_err(|_| {
        AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.1-resolve-login-flows-unavailable",
        }
    })?;
    Ok(response
        .flows
        .iter()
        .map(|flow| match flow {
            LoginType::Token(token) => LoginFlow::token(token.get_login_token),
            other => LoginFlow::from_matrix_parts(other.login_type(), None),
        })
        .collect())
}

fn validate_input(raw: &str) -> Result<&str, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-resolve-empty-input",
            reason: "server name or homeserver url is empty",
        });
    }
    if trimmed.chars().count() > MAX_INPUT_CHARS
        || trimmed.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-resolve-invalid-input",
            reason: "server name or homeserver url is malformed",
        });
    }
    Ok(trimmed)
}

/// Discovery failures never carry SDK text; only the category survives.
fn map_build_error(error: ClientBuilderError) -> AuthError {
    match error.to_factory_error().category {
        MatrixIpcErrorCategory::Connectivity => AuthError::Connectivity {
            diagnostic_id: "p3.1-resolve-connectivity",
        },
        _ => AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.1-resolve-homeserver-unavailable",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::LoginFlowKind;
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    async fn mount_login_types(server: &MockServer, flows: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "flows": flows
            })))
            .mount(server)
            .await;
    }

    fn input_for(server: &MatrixMockServer) -> String {
        // Bare `host:port` makes the SDK try well-known first, as a typed
        // server name would. The mock speaks http, so name the scheme.
        format!("http://{}", server.server().address())
    }

    #[tokio::test]
    async fn well_known_delegation_resolves_to_the_delegated_homeserver() {
        let delegating = MatrixMockServer::new().await;
        let homeserver = MatrixMockServer::new().await;
        delegating
            .mock_well_known()
            .ok_with_homeserver_url(&homeserver.uri())
            .mount()
            .await;
        homeserver
            .mock_versions()
            .with_simplified_sliding_sync()
            .ok()
            .mount()
            .await;
        mount_login_types(
            homeserver.server(),
            serde_json::json!([
                {"type": "m.login.password"},
                {"type": "m.login.sso"},
                {"type": "m.login.token", "get_login_token": true}
            ]),
        )
        .await;

        let resolved = resolve_homeserver(&input_for(&delegating)).await.unwrap();

        assert_eq!(resolved.homeserver_url, homeserver.uri());
        assert!(resolved.native_sliding_sync);
        let kinds: Vec<_> = resolved.login_flows.iter().map(|flow| flow.kind).collect();
        assert_eq!(
            kinds,
            [
                LoginFlowKind::Password,
                LoginFlowKind::Unknown,
                LoginFlowKind::Token
            ]
        );
        assert_eq!(resolved.login_flows[1].matrix_type, "m.login.sso");
        assert_eq!(resolved.login_flows[2].get_login_token, Some(true));
        let delegating_paths: Vec<_> = delegating
            .server()
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| request.url.path().to_owned())
            .collect();
        assert_eq!(
            delegating_paths,
            ["/.well-known/matrix/client"],
            "only discovery hits the delegating host"
        );
    }

    #[tokio::test]
    async fn without_well_known_the_input_is_used_as_the_homeserver_url() {
        let server = MatrixMockServer::new().await;
        server.mock_well_known().error404().mount().await;
        server
            .mock_versions()
            .with_simplified_sliding_sync()
            .ok()
            .mount()
            .await;
        mount_login_types(
            server.server(),
            serde_json::json!([{"type": "m.login.password"}]),
        )
        .await;

        let resolved = resolve_homeserver(&format!("{}/", input_for(&server)))
            .await
            .unwrap();

        assert_eq!(resolved.homeserver_url, server.uri());
        assert!(resolved.native_sliding_sync);
        assert_eq!(resolved.login_flows, [LoginFlow::password()]);
    }

    #[tokio::test]
    async fn a_server_without_simplified_sliding_sync_is_reported() {
        let server = MatrixMockServer::new().await;
        server.mock_well_known().error404().mount().await;
        // Older proxy markers are not what the SDK syncs with.
        server
            .mock_versions()
            .with_feature("org.matrix.msc3575", true)
            .ok()
            .mount()
            .await;
        mount_login_types(
            server.server(),
            serde_json::json!([{"type": "m.login.password"}]),
        )
        .await;

        let resolved = resolve_homeserver(&input_for(&server)).await.unwrap();

        assert!(!resolved.native_sliding_sync);
        assert_eq!(resolved.login_flows, [LoginFlow::password()]);
    }

    #[tokio::test]
    async fn unreachable_or_non_matrix_input_fails_without_leaking_it() {
        let server = MatrixMockServer::new().await;
        let input = input_for(&server);
        let error = resolve_homeserver(&input).await.unwrap_err();
        assert!(matches!(error, AuthError::HomeserverUnavailable { .. }));
        assert!(error.display_is_privacy_safe(&[&input, "127.0.0.1"]));

        for bad in ["", "   ", "matrix org", "matrix.org\u{0}"] {
            assert!(matches!(
                resolve_homeserver(bad).await,
                Err(AuthError::InvalidInput { .. })
            ));
        }
    }
}
