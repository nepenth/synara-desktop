//! Shared, credential-free Matrix login-flow discovery domain and transport.
//!
//! This module owns only URL normalization, login-type discovery, and its
//! bounded read-only HTTP client. It neither accepts nor returns credentials.

mod error;
mod http_transport;
mod input;
mod login_flow;

pub use error::AuthError;
pub use http_transport::{
    parse_login_types_json, HttpLoginFlowTransport, AUTH_HTTP_MAX_RESPONSE_BYTES,
    AUTH_HTTP_TIMEOUT_SECS,
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
