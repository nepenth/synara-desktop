//! P3.1 — Discovery and login-flow service foundation.
//!
//! Harness / foundation only until cutover:
//! - homeserver URL + server-name input normalization
//! - well-known discovery behind [`DiscoveryTransport`] (mockable)
//! - login-flow discovery behind [`LoginFlowTransport`] (mockable)
//! - stable Synara domain types (not raw SDK / Ruma on the boundary)
//! - optional thin bridge into P2.3 client-builder identity/homeserver URL
//!
//! **Out of scope:** password/token login (P3.2), SSO callback (P3.3), UIA
//! (P3.4), refresh-token persistence (P3.5), session restore (P3.6), production
//! Matrix Tauri commands, dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p3.1-discovery-login-flow.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod client_config;
mod discovery;
mod error;
mod input;
mod login_flow;

pub use client_config::{homeserver_url_for_client_builder, identity_with_discovered_homeserver};
pub use discovery::{
    discover_homeserver, discover_homeserver_and_login_flows, DiscoveryResult, DiscoveryTransport,
    MockDiscoveryTransport, WellKnownClientConfig,
};
pub use error::AuthError;
pub use input::{
    normalize_homeserver_url, normalize_server_name, parse_discovery_input, DiscoveryInput,
    DiscoveryInputKind, NormalizedHomeserverUrl, NormalizedServerName,
};
pub use login_flow::{
    discover_login_flows, map_matrix_login_types, LoginFlow, LoginFlowDiscoveryResult,
    LoginFlowKind, LoginFlowTransport, MockLoginFlowTransport, SsoIdentityProvider,
};

/// Static marker for link / schema smoke (no network, no Client, no login).
pub const MATRIX_AUTH_MARKER: &str = "matrix-auth-discovery-login-flow-p3.1";

/// Touch auth/discovery foundation paths so they remain linked in non-test builds.
pub fn matrix_auth_markers() -> &'static str {
    let _kinds = LoginFlowKind::ALL_KNOWN.len();
    let _password = LoginFlowKind::Password.matrix_type();
    let _input = DiscoveryInputKind::HomeserverUrl.as_str();
    debug_assert!(_kinds >= 4);
    debug_assert_eq!(_password, Some("m.login.password"));
    debug_assert_eq!(_input, "homeserver_url");
    debug_assert_eq!(MATRIX_AUTH_MARKER, "matrix-auth-discovery-login-flow-p3.1");
    MATRIX_AUTH_MARKER
}

#[cfg(test)]
mod tests;
