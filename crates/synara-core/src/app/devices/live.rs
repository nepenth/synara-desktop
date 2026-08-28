//! Live V-CRYPTO.7 device projection and session-scoped update signal.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;

use futures_util::StreamExt;
use matrix_sdk::{
    encryption::VerificationState,
    ruma::{
        api::client::uiaa::{
            AuthData, AuthType, MatrixUserIdentifier, Password, UiaaInfo, UserIdentifier,
        },
        OwnedDeviceId,
    },
    Client,
};
use serde::Serialize;
use tokio::task::JoinHandle;

use crate::app::room_keys::{
    project_room_key_status, NativeRoomKeyTransferStatus, RoomKeyTransferFlow,
};

use super::{
    sort_native_device_summaries, NativeDeviceDeleteAuthentication, NativeDeviceDeleteChallenge,
    NativeDeviceDeleteResult, NativeDeviceSnapshot, NativeDeviceSummary, NativeDeviceTrust,
    NativeOwnDeviceVerification,
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
struct DeviceDeleteState {
    pending: Option<PendingDeviceDeletion>,
    next_operation_id: u64,
}

pub struct NativeDeviceOwner {
    client: Client,
    session_generation: u64,
    task: JoinHandle<()>,
    delete: Mutex<DeviceDeleteState>,
    room_keys: Arc<AsyncMutex<RoomKeyTransferFlow>>,
    pending_cross_signing: Mutex<Option<String>>,
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
            delete: Mutex::new(DeviceDeleteState {
                pending: None,
                next_operation_id: 0,
            }),
            room_keys: Arc::new(AsyncMutex::new(RoomKeyTransferFlow::new(
                session_generation,
            ))),
            pending_cross_signing: Mutex::new(None),
        })
    }

    pub fn room_key_transfer(&self) -> Arc<AsyncMutex<RoomKeyTransferFlow>> {
        Arc::clone(&self.room_keys)
    }

    pub async fn room_key_status(&self) -> NativeRoomKeyTransferStatus {
        let flow = self.room_keys.lock().await;
        project_room_key_status(self.session_generation, &flow)
    }

    /// UI reads still go through `Client::devices`. This is not a second list.
    pub async fn snapshot(&self) -> Result<NativeDeviceSnapshot, &'static str> {
        snapshot(&self.client, self.session_generation).await
    }

    pub async fn backup_status(
        &self,
    ) -> Result<crate::app::backup::NativeBackupStatus, &'static str> {
        crate::app::backup::status(&self.client, self.session_generation).await
    }

    /// Restore encryption backup. Recovery secret is a method argument only.
    pub async fn restore_backup(
        &self,
        recovery_secret: &str,
    ) -> Result<crate::app::backup::MatrixRestoreBackupResult, &'static str> {
        crate::app::backup::restore(&self.client, self.session_generation, recovery_secret).await
    }

    pub async fn cross_signing_setup(
        &self,
    ) -> Result<crate::app::cross_signing::NativeCrossSigningSetupResult, &'static str> {
        let (result, pending) =
            crate::app::cross_signing::setup(&self.client, self.session_generation).await?;
        self.set_pending_cross_signing(pending)?;
        Ok(result)
    }

    pub fn pending_cross_signing_auth(&self) -> Result<String, &'static str> {
        self.pending_cross_signing
            .lock()
            .map_err(|_| "v-crypto.2-cross-signing-state-poisoned")?
            .clone()
            .ok_or("v-crypto.2-cross-signing-auth-not-pending")
    }

    pub fn set_pending_cross_signing(&self, pending: Option<String>) -> Result<(), &'static str> {
        *self
            .pending_cross_signing
            .lock()
            .map_err(|_| "v-crypto.2-cross-signing-state-poisoned")? = pending;
        Ok(())
    }

    pub async fn finish_cross_signing_setup(
        &self,
    ) -> Result<crate::app::cross_signing::NativeCrossSigningSetupResult, &'static str> {
        self.set_pending_cross_signing(None)?;
        crate::app::cross_signing::complete(&self.client, self.session_generation).await
    }

    /// Rename a device, then re-snapshot the live list.
    pub async fn rename(
        &self,
        device_id: &str,
        display_name: &str,
    ) -> Result<NativeDeviceSnapshot, &'static str> {
        let display_name = display_name.trim();
        if display_name.is_empty() {
            return Err("v-crypto.7-device-rename-empty");
        }
        let device_id = OwnedDeviceId::from(device_id);
        self.client
            .rename_device(&device_id, display_name)
            .await
            .map_err(|_| "v-crypto.7-device-rename-failed")?;
        self.snapshot().await
    }

    pub async fn delete_start(
        &self,
        device_ids: Vec<String>,
    ) -> Result<NativeDeviceDeleteResult, &'static str> {
        self.clear_pending()?;
        let device_ids = self.validate_deletion(device_ids).await?;
        match self.client.delete_devices(&device_ids, None).await {
            Ok(_) => self.complete_deletion(&device_ids).await,
            Err(error) => {
                let info = error
                    .as_uiaa_response()
                    .ok_or("v-crypto.7-device-delete-start-failed")?;
                self.retain_challenge(device_ids, info)
            }
        }
    }

    pub fn delete_cancel(
        &self,
        operation_id: u64,
        session_generation: u64,
    ) -> Result<(), &'static str> {
        self.validate_pending(operation_id, session_generation)?;
        self.clear_pending()
    }

    /// Clone the live pending challenge so the shell can finish password UIAA
    /// without putting the password on the Core envelope.
    pub fn pending_deletion(
        &self,
        operation_id: u64,
        session_generation: u64,
    ) -> Result<PendingDeviceDeletion, &'static str> {
        self.validate_pending(operation_id, session_generation)
    }

    /// Finish a pending delete with the account password. Password stays off
    /// `Core::command` JSON; iOS/SharedCore call this method directly.
    pub async fn authenticate_delete_password(
        &self,
        operation_id: u64,
        session_generation: u64,
        password: &str,
    ) -> Result<NativeDeviceDeleteResult, &'static str> {
        if password.is_empty() {
            return Err("v-crypto.7-device-delete-password-empty");
        }
        let pending = self.pending_deletion(operation_id, session_generation)?;
        let user_id = self
            .client
            .user_id()
            .ok_or("v-crypto.7-device-delete-user-missing")?;
        let mut auth = Password::new(
            UserIdentifier::Matrix(MatrixUserIdentifier::new(user_id.to_string())),
            password.to_owned(),
        );
        auth.session = Some(pending.auth_session.clone());
        match self
            .client
            .delete_devices(&pending.device_ids, Some(AuthData::Password(auth)))
            .await
        {
            Ok(_) => self.complete_deletion(&pending.device_ids).await,
            Err(error) => {
                let info = error
                    .as_uiaa_response()
                    .ok_or("v-crypto.7-device-delete-password-failed")?;
                let authentication_failed = !info.completed.contains(&AuthType::Password);
                self.refresh_delete_challenge(info, authentication_failed)
            }
        }
    }

    pub fn refresh_delete_challenge(
        &self,
        info: &UiaaInfo,
        authentication_failed: bool,
    ) -> Result<NativeDeviceDeleteResult, &'static str> {
        let pending = {
            let mut state = self
                .delete
                .lock()
                .map_err(|_| "v-crypto.7-device-delete-state-poisoned")?;
            state
                .pending
                .take()
                .ok_or("v-crypto.7-device-delete-not-pending")?
        };
        self.install_challenge(
            pending.operation_id,
            pending.device_ids,
            info,
            authentication_failed,
        )
    }

    pub async fn complete_deletion(
        &self,
        deleted: &[OwnedDeviceId],
    ) -> Result<NativeDeviceDeleteResult, &'static str> {
        let snapshot = self.snapshot().await?;
        if deleted
            .iter()
            .any(|device_id| snapshot.contains(device_id.as_str()))
        {
            return Err("v-crypto.7-device-delete-readback-incomplete");
        }
        self.clear_pending()?;
        Ok(NativeDeviceDeleteResult::Complete { snapshot })
    }

    fn clear_pending(&self) -> Result<(), &'static str> {
        let mut state = self
            .delete
            .lock()
            .map_err(|_| "v-crypto.7-device-delete-state-poisoned")?;
        state.pending = None;
        Ok(())
    }

    fn validate_pending(
        &self,
        operation_id: u64,
        session_generation: u64,
    ) -> Result<PendingDeviceDeletion, &'static str> {
        if self.session_generation != session_generation {
            return Err("v-crypto.7-device-delete-stale-generation");
        }
        let state = self
            .delete
            .lock()
            .map_err(|_| "v-crypto.7-device-delete-state-poisoned")?;
        let pending = state
            .pending
            .as_ref()
            .ok_or("v-crypto.7-device-delete-not-pending")?;
        if pending.session_generation != session_generation {
            return Err("v-crypto.7-device-delete-stale-generation");
        }
        if pending.operation_id != operation_id {
            return Err("v-crypto.7-device-delete-operation-mismatch");
        }
        Ok(PendingDeviceDeletion {
            operation_id: pending.operation_id,
            session_generation: pending.session_generation,
            device_ids: pending.device_ids.clone(),
            auth_session: pending.auth_session.clone(),
        })
    }

    async fn validate_deletion(
        &self,
        device_ids: Vec<String>,
    ) -> Result<Vec<OwnedDeviceId>, &'static str> {
        if device_ids.is_empty() {
            return Err("v-crypto.7-device-delete-selection-empty");
        }
        let snapshot = self.snapshot().await?;
        let current = snapshot
            .devices
            .iter()
            .find(|device| device.is_current)
            .map(|device| device.device_id.as_str())
            .ok_or("v-crypto.7-device-delete-current-missing")?;
        let mut unique = BTreeSet::new();
        for device_id in device_ids {
            if device_id.is_empty() || device_id == current || !snapshot.contains(&device_id) {
                return Err("v-crypto.7-device-delete-selection-invalid");
            }
            unique.insert(OwnedDeviceId::from(device_id));
        }
        Ok(unique.into_iter().collect())
    }

    fn retain_challenge(
        &self,
        device_ids: Vec<OwnedDeviceId>,
        info: &UiaaInfo,
    ) -> Result<NativeDeviceDeleteResult, &'static str> {
        let operation_id = {
            let mut state = self
                .delete
                .lock()
                .map_err(|_| "v-crypto.7-device-delete-state-poisoned")?;
            let operation_id = state
                .next_operation_id
                .checked_add(1)
                .ok_or("v-crypto.7-device-delete-operation-overflow")?;
            state.next_operation_id = operation_id;
            operation_id
        };
        self.install_challenge(operation_id, device_ids, info, false)
    }

    fn install_challenge(
        &self,
        operation_id: u64,
        device_ids: Vec<OwnedDeviceId>,
        info: &UiaaInfo,
        authentication_failed: bool,
    ) -> Result<NativeDeviceDeleteResult, &'static str> {
        let auth_session = info
            .session
            .clone()
            .ok_or("v-crypto.7-device-delete-auth-session-missing")?;
        let available = supported_delete_authentication(info);
        if !available.contains(&NativeDeviceDeleteAuthentication::Password) {
            return Err("v-crypto.7-device-delete-auth-unsupported");
        }
        {
            let mut state = self
                .delete
                .lock()
                .map_err(|_| "v-crypto.7-device-delete-state-poisoned")?;
            state.pending = Some(PendingDeviceDeletion {
                operation_id,
                session_generation: self.session_generation,
                device_ids,
                auth_session,
            });
        }
        Ok(NativeDeviceDeleteResult::AuthenticationRequired {
            challenge: NativeDeviceDeleteChallenge {
                operation_id,
                session_generation: self.session_generation,
                authentication: NativeDeviceDeleteAuthentication::Password,
                authentication_failed,
            },
        })
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
    let encryption = client.encryption();
    // This performs the SDK's initial own-key query before we sample the
    // subscriber, avoiding a UI-visible Unknown -> hidden-button race. A
    // failed key query is recoverable verification metadata, not permission to
    // erase the authoritative homeserver session list below.
    let current_device_id = client
        .device_id()
        .ok_or("v-crypto.7-device-snapshot-current-missing")?;
    let user_id = client
        .user_id()
        .ok_or("v-crypto.7-device-snapshot-user-missing")?;
    // Eligibility can perform an initial `/keys/query`. Run it beside the
    // authoritative homeserver session fetch and local crypto enrichment so a
    // slow authority lookup cannot hold the entire Sessions screen hostage.
    let (eligibility, server_devices, crypto_devices) = tokio::join!(
        tokio::time::timeout(
            Duration::from_secs(8),
            encryption.has_devices_to_verify_against(),
        ),
        client.devices(),
        encryption.get_user_devices(user_id),
    );
    let has_devices_to_verify_against = match eligibility {
        Ok(Ok(has_devices)) => Some(has_devices),
        Ok(Err(_)) | Err(_) => None,
    };
    let own_verification = match encryption.verification_state().get() {
        VerificationState::Unknown => NativeOwnDeviceVerification::Unknown,
        VerificationState::Unverified => NativeOwnDeviceVerification::Unverified,
        VerificationState::Verified => NativeOwnDeviceVerification::Verified,
    };
    let server_devices = server_devices.map_err(|_| "v-crypto.7-device-snapshot-server-failed")?;
    // The homeserver session list is authoritative for account/device actions.
    // Crypto trust enrichment may be temporarily unavailable while a fresh
    // store is still processing device keys; do not erase valid sessions in
    // that case or the user loses the only path to target verification.
    let crypto_devices = crypto_devices.ok();

    let mut devices = server_devices
        .devices
        .into_iter()
        .map(|device| {
            let trust = crypto_devices
                .as_ref()
                .and_then(|devices| devices.get(&device.device_id))
                .map(|crypto_device| {
                    // A completed direct SAS marks the peer locally trusted.
                    // `is_verified()` deliberately includes that SDK-owned
                    // trust as well as cross-signing trust; limiting this
                    // projection to cross-signing made a successful SAS appear
                    // unverified everywhere in the product after the sheet
                    // reported completion.
                    if crypto_device.is_verified() {
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
        own_verification,
        has_devices_to_verify_against,
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
