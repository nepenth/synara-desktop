//! P3.1 / P3.2 / P3.3 / P3.4 / R0.7 — Discovery, login-flow, password/token, SSO, UIA.
//!
//! - homeserver URL + server-name input normalization
//! - well-known discovery behind [`DiscoveryTransport`] (mock + live HTTP)
//! - login-flow discovery behind [`LoginFlowTransport`] (mock + live HTTP)
//! - password + token login against an unauthenticated P2.3 SDK client
//! - SSO / OAuth callback lifecycle coordinator (no tokens stored)
//! - interactive auth (UIA) multi-stage coordinator (no secrets stored)
//! - platform device display names (`Synara macOS` / `Synara Linux` / …)
//! - stable Synara domain types (not raw SDK / Ruma on the boundary)
//! - optional thin bridge into P2.3 client-builder identity/homeserver URL
//!
//! **Out of scope:** session restore (P3.6), production Matrix Tauri commands,
//! dual-backend, dual sync.
//! Session secret persist after login lives in [`crate::matrix::lifecycle`] (P3.5).
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p3.1-discovery-login-flow.md`
//! - `docs/matrix-rust-sdk/p3.2-password-token-login.md`
//! - `docs/matrix-rust-sdk/p3.3-sso-callback.md`
//! - `docs/matrix-rust-sdk/p3.4-uia.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod client_config;
mod device_name;
mod discovery;
mod error;
mod http_transport;
mod input;
mod login;
mod login_flow;
mod sso_callback;
mod uia;

pub use client_config::{homeserver_url_for_client_builder, identity_with_discovered_homeserver};
pub use device_name::{
    host_device_platform, platform_device_display_name, DevicePlatform,
    DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK, DEVICE_DISPLAY_NAME_IOS, DEVICE_DISPLAY_NAME_LINUX,
    DEVICE_DISPLAY_NAME_MACOS,
};
pub use discovery::{
    discover_homeserver, discover_homeserver_and_login_flows, DiscoveryResult, DiscoveryTransport,
    MockDiscoveryTransport, WellKnownClientConfig,
};
pub use error::AuthError;
pub use http_transport::{
    parse_login_types_json, parse_well_known_client_json, HttpDiscoveryTransport,
    HttpLoginFlowTransport, AUTH_HTTP_TIMEOUT_SECS,
};
pub use input::{
    normalize_homeserver_url, normalize_server_name, parse_discovery_input, DiscoveryInput,
    DiscoveryInputKind, NormalizedHomeserverUrl, NormalizedServerName,
};
pub use login::{
    login_with_password, login_with_token, LoginMethodKind, LoginOptions, LoginResult,
};
pub use login_flow::{
    discover_login_flows, map_matrix_login_types, LoginFlow, LoginFlowDiscoveryResult,
    LoginFlowKind, LoginFlowTransport, MockLoginFlowTransport, SsoIdentityProvider,
};
pub use sso_callback::{
    SsoCallbackFlow, SsoCallbackOutcome, SsoCallbackPhase, MAX_PENDING_SSO_FLOWS,
    MAX_REDIRECT_URL_CHARS, MAX_SSO_ID_CHARS,
};
pub use uia::{
    UiaFlowKind, UiaOutcome, UiaPhase, UiaSession, UiaStage, UiaStageKind, MAX_UIA_ID_CHARS,
    MAX_UIA_STAGES,
};

/// Static marker for link / schema smoke (no network, no Client, no login).
pub const MATRIX_AUTH_MARKER: &str = "matrix-auth-sso-p3.3+uia-p3.4";

/// Touch auth foundation paths so they remain linked in non-test builds.
pub fn matrix_auth_markers() -> &'static str {
    let _kinds = LoginFlowKind::ALL_KNOWN.len();
    let _password = LoginFlowKind::Password.matrix_type();
    let _input = DiscoveryInputKind::HomeserverUrl.as_str();
    let _timeout = AUTH_HTTP_TIMEOUT_SECS;
    let _device = platform_device_display_name();
    let _method = LoginMethodKind::Password.as_str();
    let sso = SsoCallbackFlow::new(0);
    debug_assert!(!sso.is_active());
    debug_assert!(sso.never_stores_tokens());
    debug_assert_eq!(SsoCallbackPhase::Idle.as_str(), "idle");
    let uia = UiaSession::new(0);
    debug_assert!(!uia.is_active());
    debug_assert!(uia.never_stores_secrets());
    debug_assert_eq!(UiaPhase::Idle.as_str(), "idle");
    debug_assert_eq!(
        UiaStageKind::from_matrix_type("m.login.password"),
        UiaStageKind::Password
    );
    debug_assert!(_kinds >= 4);
    debug_assert_eq!(_password, Some("m.login.password"));
    debug_assert_eq!(_input, "homeserver_url");
    debug_assert!(_timeout >= 5);
    debug_assert!(_device.starts_with("Synara "));
    debug_assert_eq!(_method, "password");
    debug_assert_eq!(MATRIX_AUTH_MARKER, "matrix-auth-sso-p3.3+uia-p3.4");
    MATRIX_AUTH_MARKER
}

#[cfg(test)]
mod tests;
