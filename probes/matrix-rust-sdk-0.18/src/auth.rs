//! Auth, discovery, and session-restore compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::authentication::AuthSession;
use matrix_sdk::authentication::matrix::{LoginBuilder, MatrixAuth};
use matrix_sdk::ruma::api::client::session::get_login_types;
use matrix_sdk::{Client, ClientBuilder, ServerName};

/// P0.3b-client-builder-homeserver-url — `ClientBuilder::homeserver_url`.
///
/// Source: `crates/matrix-sdk/src/client/builder/mod.rs` (`pub fn homeserver_url`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_builder_homeserver_url() {
    fn _shape(b: ClientBuilder) -> ClientBuilder {
        b.homeserver_url("https://example.invalid")
    }
    let _ = _shape;
}

/// P0.3b-client-builder-server-name — `ClientBuilder::server_name`.
///
/// Source: `crates/matrix-sdk/src/client/builder/mod.rs` (`pub fn server_name`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_builder_server_name() {
    fn _shape(b: ClientBuilder, name: &ServerName) -> ClientBuilder {
        b.server_name(name)
    }
    let _ = _shape;
}

/// P0.3b-client-builder-server-name-or-url — `ClientBuilder::server_name_or_homeserver_url`.
///
/// Source: `crates/matrix-sdk/src/client/builder/mod.rs`
/// (`pub fn server_name_or_homeserver_url`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_builder_server_name_or_url() {
    fn _shape(b: ClientBuilder) -> ClientBuilder {
        b.server_name_or_homeserver_url("example.invalid")
    }
    let _ = _shape;
}

/// P0.3b-auth-session-type — `matrix_sdk::AuthSession` is a public type.
///
/// Source: `crates/matrix-sdk/src/authentication/mod.rs` (`pub enum AuthSession`)
/// and crate-root re-export.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_auth_session_type() -> &'static str {
    std::any::type_name::<AuthSession>()
}

/// P0.3b-client-matrix-auth — `Client::matrix_auth() -> MatrixAuth`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn matrix_auth`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_matrix_auth() {
    fn _shape(client: &Client) -> MatrixAuth {
        client.matrix_auth()
    }
    let _ = _shape;
}

/// P0.3b-matrix-auth-login-username — `MatrixAuth::login_username` → `LoginBuilder`.
///
/// Source: `crates/matrix-sdk/src/authentication/matrix/mod.rs`
/// (`pub fn login_username`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_matrix_auth_login_username() {
    fn _shape(auth: &MatrixAuth, id: &str, password: &str) -> LoginBuilder {
        auth.login_username(id, password)
    }
    let _ = _shape;
}

/// P0.3b-matrix-auth-get-login-types — `MatrixAuth::get_login_types`.
///
/// Source: `crates/matrix-sdk/src/authentication/matrix/mod.rs`
/// (`pub async fn get_login_types`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_matrix_auth_get_login_types() {
    async fn _shape(auth: &MatrixAuth) -> matrix_sdk::HttpResult<get_login_types::v3::Response> {
        auth.get_login_types().await
    }
    let _ = _shape;
}

/// P0.3b-client-restore-session — `Client::restore_session`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn restore_session`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_restore_session() {
    async fn _shape(client: &Client, session: AuthSession) -> matrix_sdk::Result<()> {
        client.restore_session(session).await
    }
    let _ = _shape;
}

/// P0.3b-client-logout — `Client::logout`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn logout`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_logout() {
    async fn _shape(client: &Client) -> matrix_sdk::Result<(), matrix_sdk::Error> {
        client.logout().await
    }
    let _ = _shape;
}

/// Run every auth/discovery probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_client_builder_homeserver_url();
    probe_client_builder_server_name();
    probe_client_builder_server_name_or_url();
    let _ = probe_auth_session_type();
    probe_client_matrix_auth();
    probe_matrix_auth_login_username();
    probe_matrix_auth_get_login_types();
    probe_client_restore_session();
    probe_client_logout();
}
