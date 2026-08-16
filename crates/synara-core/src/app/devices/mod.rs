//! Credential-free V-CRYPTO.7 device presentation plus live device-list owner.
//!
//! Shells supply the emit sink (desktop Tauri event / later iOS UniFFI).
//! Password UIAA continuation stays in the desktop shell so the password
//! never crosses `Core::command`. Start/cancel route through the owner.

use serde::{Deserialize, Serialize};

mod live;
pub use live::{
    snapshot, supported_delete_authentication, DeviceListUpdateEmit, NativeDeviceOwner,
    NativeDeviceUpdateSignal, PendingDeviceDeletion,
};

/// Tauri event: device list may have changed; UI re-snapshots via matrix_get_* commands.
/// Signal only — never carries device keys or tokens.
pub const DEVICE_LIST_UPDATED_EVENT: &str = "matrix-device-list-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDeviceTrust {
    Verified,
    Unverified,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceSummary {
    pub device_id: String,
    pub display_name: Option<String>,
    pub last_seen_ip: Option<String>,
    pub last_seen_ts: Option<u64>,
    pub trust: NativeDeviceTrust,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceSnapshot {
    pub session_generation: u64,
    pub devices: Vec<NativeDeviceSummary>,
}

impl NativeDeviceSnapshot {
    pub fn contains(&self, device_id: &str) -> bool {
        self.devices
            .iter()
            .any(|device| device.device_id == device_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDeviceDeleteAuthentication {
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceDeleteChallenge {
    pub operation_id: u64,
    pub session_generation: u64,
    pub authentication: NativeDeviceDeleteAuthentication,
    pub authentication_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum NativeDeviceDeleteResult {
    Complete {
        snapshot: NativeDeviceSnapshot,
    },
    AuthenticationRequired {
        challenge: NativeDeviceDeleteChallenge,
    },
}

/// Current first; other devices retain product parity by most-recent
/// activity descending, with a deterministic ID tiebreaker.
pub fn sort_native_device_summaries(devices: &mut [NativeDeviceSummary]) {
    devices.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| right.last_seen_ts.cmp(&left.last_seen_ts))
            .then_with(|| left.device_id.cmp(&right.device_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_projection_contains_presentation_fields_but_no_device_keys() {
        let snapshot = NativeDeviceSnapshot {
            session_generation: 7,
            devices: vec![NativeDeviceSummary {
                device_id: "DEVICE".into(),
                display_name: Some("Synara macOS".into()),
                last_seen_ip: Some("192.0.2.1".into()),
                last_seen_ts: Some(1),
                trust: NativeDeviceTrust::Verified,
                is_current: true,
            }],
        };
        let json = serde_json::to_string(&snapshot)
            .unwrap()
            .to_ascii_lowercase();
        assert!(json.contains("lastseenip"));
        for forbidden in [
            "access_token",
            "refresh_token",
            "device_key",
            "ed25519",
            "curve25519",
            "password",
            "auth_session",
        ] {
            assert!(!json.contains(forbidden));
        }

        let challenge = NativeDeviceDeleteChallenge {
            operation_id: 3,
            session_generation: 7,
            authentication: NativeDeviceDeleteAuthentication::Password,
            authentication_failed: false,
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        assert!(challenge_json.contains(r#""authentication":"password""#));
        assert!(!challenge_json.contains(r#""authentication":["#));
    }

    #[test]
    fn sort_puts_current_first_then_recent_then_id() {
        let mut devices = vec![
            NativeDeviceSummary {
                device_id: "B".into(),
                display_name: None,
                last_seen_ip: None,
                last_seen_ts: Some(1),
                trust: NativeDeviceTrust::Unverified,
                is_current: false,
            },
            NativeDeviceSummary {
                device_id: "A".into(),
                display_name: None,
                last_seen_ip: None,
                last_seen_ts: Some(1),
                trust: NativeDeviceTrust::Unverified,
                is_current: false,
            },
            NativeDeviceSummary {
                device_id: "CUR".into(),
                display_name: None,
                last_seen_ip: None,
                last_seen_ts: Some(0),
                trust: NativeDeviceTrust::Verified,
                is_current: true,
            },
        ];
        sort_native_device_summaries(&mut devices);
        assert_eq!(devices[0].device_id, "CUR");
        assert_eq!(devices[1].device_id, "A");
        assert_eq!(devices[2].device_id, "B");
        assert!(NativeDeviceSnapshot {
            session_generation: 1,
            devices,
        }
        .contains("CUR"));
    }
}
