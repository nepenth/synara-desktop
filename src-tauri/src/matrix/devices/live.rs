//! Live V-CRYPTO.7 device projection and session-scoped update signal.

use std::collections::BTreeSet;

use futures_util::StreamExt;
use matrix_sdk::{
    ruma::{
        api::client::uiaa::{AuthType, UiaaInfo},
        OwnedDeviceId,
    },
    Client,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;

pub const DEVICE_LIST_UPDATED_EVENT: &str = "matrix-device-list-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDeviceTrust {
    Verified,
    Unverified,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceSummary {
    pub device_id: String,
    pub display_name: Option<String>,
    pub last_seen_ip: Option<String>,
    pub last_seen_ts: Option<u64>,
    pub trust: NativeDeviceTrust,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDeviceDeleteAuthentication {
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceDeleteChallenge {
    pub operation_id: u64,
    pub session_generation: u64,
    pub authentication: NativeDeviceDeleteAuthentication,
    pub authentication_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum NativeDeviceDeleteResult {
    Complete {
        snapshot: NativeDeviceSnapshot,
    },
    AuthenticationRequired {
        challenge: NativeDeviceDeleteChallenge,
    },
}

pub struct PendingDeviceDeletion {
    pub operation_id: u64,
    pub session_generation: u64,
    pub device_ids: Vec<OwnedDeviceId>,
    pub auth_session: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeDeviceUpdateSignal {
    session_generation: u64,
}

/// Owns the supported high-level crypto-device update stream for one session.
///
/// The stream is only a trigger. Every UI read still goes through
/// `Client::devices`, so it never becomes a second device-list owner.
pub struct NativeDeviceOwner {
    task: JoinHandle<()>,
}

impl NativeDeviceOwner {
    pub async fn start(
        client: &Client,
        app: AppHandle,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        let user_id = client
            .user_id()
            .ok_or("v-crypto.7-device-owner-user-missing")?
            .to_owned();
        let mut updates = client
            .encryption()
            .devices_stream()
            .await
            .map_err(|_| "v-crypto.7-device-owner-stream-unavailable")?;
        let task = tokio::spawn(async move {
            while let Some(update) = updates.next().await {
                // The SDK's supported public stream documents new/changed
                // devices only. Empty/undocumented deletion wakeups are not
                // used as an authority signal.
                if update.new.contains_key(&user_id) || update.changed.contains_key(&user_id) {
                    let _ = app.emit(
                        DEVICE_LIST_UPDATED_EVENT,
                        NativeDeviceUpdateSignal { session_generation },
                    );
                }
            }
        });
        Ok(Self { task })
    }
}

impl Drop for NativeDeviceOwner {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn snapshot(
    client: &Client,
    session_generation: u64,
) -> Result<NativeDeviceSnapshot, &'static str> {
    let current_device_id = client
        .device_id()
        .ok_or("v-crypto.7-device-snapshot-current-missing")?;
    let user_id = client
        .user_id()
        .ok_or("v-crypto.7-device-snapshot-user-missing")?;
    let server_devices = client
        .devices()
        .await
        .map_err(|_| "v-crypto.7-device-snapshot-server-failed")?;
    let crypto_devices = client
        .encryption()
        .get_user_devices(user_id)
        .await
        .map_err(|_| "v-crypto.7-device-snapshot-trust-failed")?;

    let mut devices = server_devices
        .devices
        .into_iter()
        .map(|device| {
            let trust = crypto_devices
                .get(&device.device_id)
                .map(|crypto_device| {
                    if crypto_device.is_verified_with_cross_signing() {
                        NativeDeviceTrust::Verified
                    } else {
                        NativeDeviceTrust::Unverified
                    }
                })
                .unwrap_or(NativeDeviceTrust::Unsupported);
            NativeDeviceSummary {
                is_current: device.device_id == current_device_id,
                device_id: device.device_id.to_string(),
                display_name: device.display_name,
                last_seen_ip: device.last_seen_ip,
                last_seen_ts: device.last_seen_ts.map(|timestamp| u64::from(timestamp.0)),
                trust,
            }
        })
        .collect::<Vec<_>>();

    // Current first; other devices retain product parity by most-recent
    // activity descending, with a deterministic ID tiebreaker.
    devices.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| right.last_seen_ts.cmp(&left.last_seen_ts))
            .then_with(|| left.device_id.cmp(&right.device_id))
    });

    Ok(NativeDeviceSnapshot {
        session_generation,
        devices,
    })
}

pub fn supported_delete_authentication(
    info: &UiaaInfo,
) -> BTreeSet<NativeDeviceDeleteAuthentication> {
    let mut supported = BTreeSet::new();
    for flow in &info.flows {
        let remaining = flow
            .stages
            .iter()
            .filter(|stage| !info.completed.contains(stage))
            .collect::<Vec<_>>();
        if remaining.is_empty() || !remaining.iter().all(|stage| **stage == AuthType::Password) {
            continue;
        }
        supported.insert(NativeDeviceDeleteAuthentication::Password);
    }
    supported
}

#[cfg(test)]
mod tests {
    use matrix_sdk::ruma::api::client::uiaa::{AuthFlow, AuthType, UiaaInfo};

    use super::{
        supported_delete_authentication, NativeDeviceDeleteAuthentication,
        NativeDeviceDeleteChallenge, NativeDeviceSnapshot, NativeDeviceSummary, NativeDeviceTrust,
    };

    #[test]
    fn deletion_auth_projection_supports_password_only_flows() {
        let info = UiaaInfo::new(vec![
            AuthFlow::new(vec![AuthType::Password]),
            AuthFlow::new(vec![AuthType::Sso]),
            AuthFlow::new(vec![AuthType::ReCaptcha]),
        ]);
        let methods = supported_delete_authentication(&info);
        assert_eq!(methods.len(), 1);
        assert!(methods.contains(&NativeDeviceDeleteAuthentication::Password));

        let mixed = UiaaInfo::new(vec![AuthFlow::new(vec![AuthType::Password, AuthType::Sso])]);
        assert!(supported_delete_authentication(&mixed).is_empty());

        let sso_only = UiaaInfo::new(vec![AuthFlow::new(vec![AuthType::Sso])]);
        assert!(supported_delete_authentication(&sso_only).is_empty());
    }

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
}
