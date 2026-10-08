//! The account a persisted session belongs to.

use serde::{Deserialize, Serialize};

/// Non-secret account locator for one persisted session: who, which device,
/// which homeserver. Desktop writes it to `active-session.json` and returns it
/// over IPC as `MatrixLoginIdentity`, so the serde shape is a wire contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionLocator {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

#[cfg(test)]
mod tests {
    use super::SessionLocator;

    #[test]
    fn locator_wire_shape_is_camel_case_and_closed() {
        let locator = SessionLocator {
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICE".to_owned(),
            homeserver_url: "https://matrix.example.org".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(&locator).unwrap(),
            serde_json::json!({
                "userId": "@alice:example.org",
                "deviceId": "DEVICE",
                "homeserverUrl": "https://matrix.example.org",
            })
        );
        let extra = serde_json::json!({
            "userId": "@alice:example.org",
            "deviceId": "DEVICE",
            "homeserverUrl": "https://matrix.example.org",
            "accessToken": "secret",
        });
        assert!(serde_json::from_value::<SessionLocator>(extra).is_err());
    }
}
