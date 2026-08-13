//! Shared, credential-free Matrix auth domain and read-only transports.
//!
//! Owns input normalization, well-known discovery, login-type / register
//! probes, the UIA coordinator, device display names, and the identity
//! bridge. It neither accepts nor returns credentials. Live password login,
//! session product commands, and the well-known HTTP adapter stay in the shell.

mod client_config;
mod device_name;
mod discovery;
mod error;
mod http_transport;
mod input;
mod login_flow;
mod register_flow;
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
    parse_login_types_json, HttpLoginFlowTransport, HttpRegisterFlowTransport,
    AUTH_HTTP_MAX_RESPONSE_BYTES, AUTH_HTTP_TIMEOUT_SECS,
};
pub use input::{
    normalize_homeserver_url, normalize_server_name, parse_discovery_input, DiscoveryInput,
    DiscoveryInputKind, NormalizedHomeserverUrl, NormalizedServerName,
};
pub use login_flow::{
    discover_login_flows, login_flows_response, map_matrix_login_types, LoginFlow,
    LoginFlowDiscoveryResult, LoginFlowKind, LoginFlowTransport, MatrixLoginFlowDto,
    MatrixLoginFlowsResponse, MockLoginFlowTransport,
};
pub use register_flow::{
    has_unsupported_only_register_flows, parse_register_uiaa_json, probe_register_flows,
    RegisterFlowsProbe, RegisterFlowsTransport, RegisterUiaFlow, SUPPORTED_REGISTER_STAGES,
};
pub use uia::{
    UiaFlowKind, UiaOutcome, UiaPhase, UiaSession, UiaStage, UiaStageKind, MAX_UIA_ID_CHARS,
    MAX_UIA_STAGES,
};
