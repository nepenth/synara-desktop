#[cfg(test)]
use super::error::AuthError;
#[cfg(test)]
use crate::matrix::client_builder::default_user_agent;
#[cfg(test)]
pub use synara_core::app::auth::{parse_well_known_client_json, HttpDiscoveryTransport};

#[cfg(test)]
/// Product well-known transport (desktop user-agent).
pub fn product_http_discovery_transport() -> Result<HttpDiscoveryTransport, AuthError> {
    HttpDiscoveryTransport::new_with_user_agent(default_user_agent())
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_core::app::auth::AuthError;

    #[test]
    fn parse_well_known_minimal() {
        let cfg = parse_well_known_client_json(
            r#"{"m.homeserver":{"base_url":"https://matrix.example.org"}}"#,
        )
        .unwrap();
        assert_eq!(cfg.homeserver_base_url, "https://matrix.example.org");
        assert!(cfg.identity_server_base_url.is_none());
    }

    #[test]
    fn parse_well_known_with_identity() {
        let cfg = parse_well_known_client_json(
            r#"{"m.homeserver":{"base_url":"https://hs.example.org/"},"m.identity_server":{"base_url":"https://id.example.org"}}"#,
        )
        .unwrap();
        assert_eq!(cfg.homeserver_base_url, "https://hs.example.org");
        assert_eq!(
            cfg.identity_server_base_url.as_deref(),
            Some("https://id.example.org")
        );
    }

    #[test]
    fn parse_well_known_missing_homeserver() {
        assert!(matches!(
            parse_well_known_client_json(r#"{"m.identity_server":{"base_url":"https://x"}}"#),
            Err(AuthError::UnsupportedCapability {
                diagnostic_id: "r0.7-well-known-missing-homeserver"
            })
        ));
    }

    #[test]
    fn discovery_transport_constructor_does_not_panic() {
        let _ = product_http_discovery_transport();
        let _ = HttpDiscoveryTransport::new();
    }
}
