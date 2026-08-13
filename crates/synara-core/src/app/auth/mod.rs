//! Shared, credential-free Matrix login-flow discovery domain and transport.
//!
//! This module owns only URL normalization, login-type discovery, and its
//! bounded read-only HTTP client. It neither accepts nor returns credentials.

mod device_name;
mod error;
mod http_transport;
mod input;
mod login_flow;
mod register_flow;

pub use device_name::{
    host_device_platform, platform_device_display_name, DevicePlatform,
    DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK, DEVICE_DISPLAY_NAME_IOS, DEVICE_DISPLAY_NAME_LINUX,
    DEVICE_DISPLAY_NAME_MACOS,
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
