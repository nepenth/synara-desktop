//! Desktop compatibility re-export for shared login-flow discovery.

pub use synara_core::app::auth::{
    discover_login_flows, login_flows_response, map_matrix_login_types, LoginFlow,
    LoginFlowDiscoveryResult, LoginFlowKind, LoginFlowTransport, MatrixLoginFlowDto,
    MatrixLoginFlowsResponse, MockLoginFlowTransport,
};
