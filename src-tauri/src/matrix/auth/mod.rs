//! P3.1 / P3.2 / P3.4 / R0.7 / V-AUTH.2 — Discovery, login-flow, password login, UIA.
//!
//! - homeserver URL + server-name input normalization
//! - well-known discovery behind [`DiscoveryTransport`] (mock + live HTTP)
//! - login-flow discovery behind [`LoginFlowTransport`] (mock + live HTTP)
//! - password login / register / reset live in synara-core; this shell
//!   keeps Tauri product commands and Keychain vault wiring
//! - interactive auth (UIA) multi-stage coordinator (no secrets stored)
//! - platform device display names (`Synara macOS` / `Synara Linux` / …)
//! - stable Synara domain types (not raw SDK / Ruma on the boundary)
//! - optional thin bridge into P2.3 client-builder identity/homeserver URL
//! - D0.1 production Tauri password-login/session ownership
//!
//! Desktop product login is **password-only** after **V-AUTH.2** (no `m.login.token`
//! product path; SSO token-completion was removed with V-AUTH.1). Login-flow
//! discovery may still report `m.login.token` when a homeserver advertises it.
//!
//! **Out of scope:** dual-backend, dual sync, rooms, timelines.
//! Session secret persist/restore lives in [`crate::matrix::lifecycle`].
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p3.1-discovery-login-flow.md`
//! - `docs/matrix-rust-sdk/p3.2-password-token-login.md`
//! - `docs/matrix-rust-sdk/v-auth-2-token-login.md`
//! - `docs/matrix-rust-sdk/p3.4-uia.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod http_transport;
mod input;
mod login_flow;
pub(crate) mod product;

pub use error::AuthError;
pub use http_transport::{
    parse_login_types_json, parse_well_known_client_json, HttpDiscoveryTransport,
    HttpLoginFlowTransport, AUTH_HTTP_TIMEOUT_SECS,
};
pub use input::{
    normalize_homeserver_url, normalize_server_name, parse_discovery_input, DiscoveryInput,
    DiscoveryInputKind, NormalizedHomeserverUrl, NormalizedServerName,
};
pub use login_flow::{
    discover_login_flows, login_flows_response, map_matrix_login_types, LoginFlow,
    LoginFlowDiscoveryResult, LoginFlowKind, LoginFlowTransport, MockLoginFlowTransport,
};
pub use product::{
    MatrixAuthCommandError, MatrixAuthState, MatrixLoginIdentity, MatrixSessionSnapshot,
};
pub use synara_core::app::auth::{
    complete_password_reset, discover_homeserver, discover_homeserver_and_login_flows,
    homeserver_url_for_client_builder, host_device_platform, identity_with_discovered_homeserver,
    login_with_password, password_reset_ephemeral_user_id, platform_device_display_name,
    register_ephemeral_user_id, register_submit, request_password_email_token,
    request_register_email_token, DevicePlatform, DiscoveryResult, DiscoveryTransport,
    LoginMethodKind, LoginOptions, LoginResult, MockDiscoveryTransport, PasswordEmailTokenResult,
    PasswordResetOutcome, RegisterAuthStage, RegisterCompleteSecrets, RegisterSubmitOutcome,
    RegisterUiaChallenge, WellKnownClientConfig, DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK,
    DEVICE_DISPLAY_NAME_IOS, DEVICE_DISPLAY_NAME_LINUX, DEVICE_DISPLAY_NAME_MACOS,
};
/// Desktop compatibility re-exports for the shared read-only registration probe.
pub use synara_core::app::auth::{
    probe_register_flows, RegisterFlowsProbe, RegisterUiaFlow, SUPPORTED_REGISTER_STAGES,
};
pub use synara_core::app::auth::{
    UiaFlowKind, UiaOutcome, UiaPhase, UiaSession, UiaStage, UiaStageKind, MAX_UIA_ID_CHARS,
    MAX_UIA_STAGES,
};

/// Static marker for link / schema smoke (no network, no Client, no login).
pub const MATRIX_AUTH_MARKER: &str = "matrix-auth-password-p3.2+uia-p3.4";

/// Touch auth foundation paths so they remain linked in non-test builds.
pub fn matrix_auth_markers() -> &'static str {
    let _kinds = LoginFlowKind::ALL_KNOWN.len();
    let _password = LoginFlowKind::Password.matrix_type();
    let _input = DiscoveryInputKind::HomeserverUrl.as_str();
    let _timeout = AUTH_HTTP_TIMEOUT_SECS;
    let _device = platform_device_display_name();
    let _method = LoginMethodKind::Password.as_str();
    let uia = UiaSession::new(0);
    debug_assert!(!uia.is_active());
    debug_assert!(uia.never_stores_secrets());
    debug_assert_eq!(UiaPhase::Idle.as_str(), "idle");
    debug_assert_eq!(
        UiaStageKind::from_matrix_type("m.login.password"),
        UiaStageKind::Password
    );
    debug_assert_eq!(_kinds, 3);
    debug_assert_eq!(_password, Some("m.login.password"));
    debug_assert_eq!(_input, "homeserver_url");
    debug_assert!(_timeout >= 5);
    debug_assert!(_device.starts_with("Synara "));
    debug_assert_eq!(_method, "password");
    debug_assert_eq!(MATRIX_AUTH_MARKER, "matrix-auth-password-p3.2+uia-p3.4");
    MATRIX_AUTH_MARKER
}

#[cfg(test)]
mod tests;
