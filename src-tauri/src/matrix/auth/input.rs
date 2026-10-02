//! Desktop compatibility re-export for shared homeserver input validation.

pub use synara_core::app::auth::normalize_homeserver_url;
#[cfg(test)]
pub use synara_core::app::auth::{
    normalize_server_name, parse_discovery_input, DiscoveryInput, DiscoveryInputKind,
};
