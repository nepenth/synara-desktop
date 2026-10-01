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

mod error;
mod http_transport;
mod input;
mod login_flow;
pub(crate) mod product;

pub use error::AuthError;

pub use input::normalize_homeserver_url;

#[cfg(test)]
pub use input::{normalize_server_name, parse_discovery_input, DiscoveryInput, DiscoveryInputKind};
#[cfg(test)]
pub use login_flow::LoginFlow;

#[cfg(test)]
pub use login_flow::{LoginFlowKind, MockLoginFlowTransport};

pub use product::MatrixAuthState;

pub use synara_core::app::auth::{
    complete_password_reset, existing_sqlite_crypto_device_id, login_with_password,
    password_reset_ephemeral_user_id, platform_device_display_name, register_ephemeral_user_id,
    register_submit, request_password_email_token, request_register_email_token, LoginOptions,
    PasswordEmailTokenResult, PasswordResetOutcome, RegisterAuthStage, RegisterCompleteSecrets,
    RegisterSubmitOutcome,
};

#[cfg(test)]
pub use synara_core::app::auth::{
    discover_homeserver, homeserver_url_for_client_builder, DiscoveryResult,
    MockDiscoveryTransport, WellKnownClientConfig,
};

/// Desktop compatibility re-exports for the shared read-only registration probe.
pub use synara_core::app::auth::{RegisterFlowsProbe, RegisterUiaFlow};

#[cfg(test)]
pub use synara_core::app::auth::{UiaPhase, UiaSession};

#[cfg(test)]
mod tests;
