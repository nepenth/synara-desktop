//! Live V-CRYPTO.7 device projection and session-scoped update signal.

use std::collections::BTreeSet;
use std::sync::Arc;

use futures_util::StreamExt;
use matrix_sdk::{
    ruma::{
        api::client::uiaa::{AuthType, UiaaInfo},
        OwnedDeviceId,
    },
    Client,
};
use serde::Serialize;
use tokio::task::JoinHandle;

use super::{
    sort_native_device_summaries, NativeDeviceDeleteAuthentication, NativeDeviceSnapshot,
    NativeDeviceSummary, NativeDeviceTrust,
};

/// Shell-supplied sink for device-list wakeups. Desktop maps this to the
/// existing Tauri event; iOS can map it to a UniFFI callback later.
pub type DeviceListUpdateEmit = Arc<dyn Fn(NativeDeviceUpdateSignal) + Send + Sync>;

pub struct PendingDeviceDeletion {
    pub operation_id: u64,
    pub session_generation: u64,
    pub device_ids: Vec<OwnedDeviceId>,
    pub auth_session: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDeviceUpdateSignal {
    pub session_generation: u64,
}

/// Owns the supported high-level crypto-device update stream for one session.
///
/// The stream is only a trigger. Every UI read still goes through
/// `Client::devices`, so it never becomes a second device-list owner.
pub struct NativeDeviceOwner {
    client: Client,
    session_generation: u64,
    task: JoinHandle<()>,
}

impl NativeDeviceOwner {
    pub async fn start(
        client: &Client,
        emit: DeviceListUpdateEmit,
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
                    emit(NativeDeviceUpdateSignal { session_generation });
                }
            }
        });
        Ok(Self {
            client: client.clone(),
            session_generation,
            task,
        })
    }

    /// UI reads still go through `Client::devices`. This is not a second list.
    pub async fn snapshot(&self) -> Result<NativeDeviceSnapshot, &'static str> {
        snapshot(&self.client, self.session_generation).await
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

    sort_native_device_summaries(&mut devices);

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

    use super::supported_delete_authentication;
    use crate::app::devices::NativeDeviceDeleteAuthentication;

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
}
