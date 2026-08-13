//! Shared Matrix auth domain: credential-free discovery plus live login /
//! register / reset orchestration.
//!
//! Owns input normalization, well-known discovery, login-type / register
//! probes, the UIA coordinator, device display names, the identity
//! bridge, read-only HTTP transports, and live password login / register /
//! password-reset against an SDK `Client`. Shells collect credentials and
//! persist tokens through the session vault trait. Tauri product commands
//! stay in the desktop shell.

mod client_config;
mod device_name;
mod discovery;
mod error;
mod http_transport;
mod input;
mod login;
mod login_flow;
mod register;
mod register_flow;
mod reset_password;
mod uia;

pub use client_config::{homeserver_url_for_client_builder, identity_with_discovered_homeserver};
pub use device_name::{
    host_device_platform, platform_device_display_name, DevicePlatform,
    DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK, DEVICE_DISPLAY_NAME_IOS, DEVICE_DISPLAY_NAME_LINUX,
    DEVICE_DISPLAY_NAME_MACOS,
};
pub use discovery::{
    discover_homeserver, discover_homeserver_and_login_flows, parse_well_known_client_json,
    DiscoveryResult, DiscoveryTransport, MockDiscoveryTransport, WellKnownClientConfig,
};
pub use error::AuthError;
pub use http_transport::{
    parse_login_types_json, HttpDiscoveryTransport, HttpLoginFlowTransport,
    HttpRegisterFlowTransport, AUTH_HTTP_MAX_RESPONSE_BYTES, AUTH_HTTP_TIMEOUT_SECS,
};
pub use input::{
    normalize_homeserver_url, normalize_server_name, parse_discovery_input, DiscoveryInput,
    DiscoveryInputKind, NormalizedHomeserverUrl, NormalizedServerName,
};
pub use login::{login_with_password, LoginMethodKind, LoginOptions, LoginResult};
pub use login_flow::{
    discover_login_flows, login_flows_response, map_matrix_login_types, LoginFlow,
    LoginFlowDiscoveryResult, LoginFlowKind, LoginFlowTransport, MatrixLoginFlowDto,
    MatrixLoginFlowsResponse, MockLoginFlowTransport,
};
pub use register::{
    register_ephemeral_user_id, register_submit, request_register_email_token, RegisterAuthStage,
    RegisterCompleteSecrets, RegisterSubmitOutcome, RegisterUiaChallenge,
};
pub use register_flow::{
    has_unsupported_only_register_flows, parse_register_uiaa_json, probe_register_flows,
    RegisterFlowsProbe, RegisterFlowsTransport, RegisterUiaFlow, SUPPORTED_REGISTER_STAGES,
};
pub use reset_password::{
    complete_password_reset, password_reset_ephemeral_user_id, request_password_email_token,
    PasswordEmailTokenResult, PasswordResetOutcome,
};
pub use uia::{
    UiaFlowKind, UiaOutcome, UiaPhase, UiaSession, UiaStage, UiaStageKind, MAX_UIA_ID_CHARS,
    MAX_UIA_STAGES,
};
