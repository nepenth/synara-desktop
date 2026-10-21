//! Live V-CRYPTO.7 device projection and session-scoped update signal.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;

use futures_util::StreamExt;
use matrix_sdk::{
    encryption::{
        backups::BackupState, identities::UserDevices, recovery::RecoveryState, VerificationState,
    },
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
    bounded_optional_hs_text, format_ed25519_fingerprint, project_native_device_trust,
    sort_native_device_summaries, NativeDeviceDeleteAuthentication, NativeDeviceDeleteChallenge,
    NativeDeviceDeleteResult, NativeDeviceSnapshot, NativeDeviceSummary, NativeDeviceTrust,
    NativeDeviceTrustSignals, NativeOwnDeviceVerification, MAX_DEVICE_DISPLAY_NAME_CHARS,
    MAX_DEVICE_LAST_SEEN_IP_CHARS,
};

/// Shell-supplied sink for device/security status wakeups. Payloads carry only
/// the generation; each shell re-reads the relevant authoritative SDK status.
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
    own_keys_query: OwnKeysQueryGate,
    server_backup: crate::app::backup::ServerBackupCache,
}

/// One item from the four SDK sources the owner merges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecuritySourceUpdate {
    /// A device-list update; `true` when it touched the user's own devices.
    Devices {
        own_user_changed: bool,
    },
    Verification(VerificationState),
    /// `None` when the backup broadcast lagged and the value is unknown.
    Backup(Option<BackupState>),
    Recovery(RecoveryState),
}

/// The security state the shells were last woken for.
///
/// The SDK re-sets its verification state after every own `/keys/query` and
/// its recovery state after every own master-key update, even when the value
/// is unchanged, and `subscribe_reset` streams replay the current value. A
/// shell's status read can cause that `/keys/query`, so forwarding every item
/// turned each read into the next wakeup. Only real transitions, and own
/// device-list changes, wake the shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SecuritySignalState {
    verification: VerificationState,
    backup: BackupState,
    recovery: RecoveryState,
}

impl SecuritySignalState {
    fn apply(&mut self, update: SecuritySourceUpdate) -> bool {
        fn replace<T: PartialEq>(slot: &mut T, value: T) -> bool {
            if *slot == value {
                return false;
            }
            *slot = value;
            true
        }
        match update {
            SecuritySourceUpdate::Devices { own_user_changed } => own_user_changed,
            SecuritySourceUpdate::Verification(state) => replace(&mut self.verification, state),
            SecuritySourceUpdate::Backup(Some(state)) => replace(&mut self.backup, state),
            SecuritySourceUpdate::Backup(None) => true,
            SecuritySourceUpdate::Recovery(state) => replace(&mut self.recovery, state),
        }
    }
}

/// Fold the merged sources into wakeups, one per real change.
fn security_wakeups(
    updates: impl futures_util::Stream<Item = SecuritySourceUpdate>,
    initial: SecuritySignalState,
) -> impl futures_util::Stream<Item = ()> {
    updates
        .scan(initial, |state, update| {
            std::future::ready(Some(state.apply(update)))
        })
        .filter_map(|changed| std::future::ready(changed.then_some(())))
}

/// Runs the SDK's own-user `/keys/query` (through
/// `has_devices_to_verify_against` or an own-identity request) once per
/// session generation instead of on every status read. After one successful
/// query the SDK keeps the own device list and identity current from sync, so
/// later reads project the same predicate over the local store. A failed or
/// timed-out query re-arms the gate; a concurrent read uses the local store.
#[derive(Debug, Default)]
pub struct OwnKeysQueryGate(AtomicU8);

impl OwnKeysQueryGate {
    const IDLE: u8 = 0;
    const RUNNING: u8 = 1;
    const DONE: u8 = 2;

    /// `true` when the caller should run the query and then call `finish`.
    pub fn try_begin(&self) -> bool {
        self.0
            .compare_exchange(
                Self::IDLE,
                Self::RUNNING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub fn finish(&self, succeeded: bool) {
        self.0.store(
            if succeeded { Self::DONE } else { Self::IDLE },
            Ordering::Release,
        );
    }
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
        let updates = client
            .encryption()
            .devices_stream()
            .await
            .map_err(|_| "v-crypto.7-device-owner-stream-unavailable")?;
        // Secret sharing can finish after SAS and device-list updates. Observe
        // the SDK's own trust, recovery and backup streams too, so a successful
        // verification does not leave the UI asking for recovery until restart.
        let encryption = client.encryption();
        let verification = encryption.verification_state();
        let backup = encryption.backups().state_stream();
        let recovery = encryption.recovery().state_stream();
        // Sample after subscribing so a transition in between is still seen.
        let initial = SecuritySignalState {
            verification: verification.get(),
            backup: encryption.backups().state(),
            recovery: encryption.recovery().state(),
        };
        let updates = futures_util::stream::select_all([
            updates
                .map(move |update| SecuritySourceUpdate::Devices {
                    own_user_changed: update.new.contains_key(&user_id)
                        || update.changed.contains_key(&user_id),
                })
                .boxed(),
            verification.map(SecuritySourceUpdate::Verification).boxed(),
            backup
                .map(|state| SecuritySourceUpdate::Backup(state.ok()))
                .boxed(),
            recovery.map(SecuritySourceUpdate::Recovery).boxed(),
        ]);
        let mut wakeups = Box::pin(security_wakeups(updates, initial));
        let task = tokio::spawn(async move {
            while wakeups.next().await.is_some() {
                emit(NativeDeviceUpdateSignal { session_generation });
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
            own_keys_query: OwnKeysQueryGate::default(),
            server_backup: crate::app::backup::ServerBackupCache::default(),
        })
    }

    /// Shared with shells that request the own identity themselves, so the
    /// session issues its own-user `/keys/query` once, not once per read.
    pub fn own_keys_query(&self) -> &OwnKeysQueryGate {
        &self.own_keys_query
    }

    pub fn room_key_transfer(&self) -> Arc<AsyncMutex<RoomKeyTransferFlow>> {
        Arc::clone(&self.room_keys)
    }

    pub async fn room_key_status(&self) -> NativeRoomKeyTransferStatus {
        let flow = self.room_keys.lock().await;
        project_room_key_status(self.session_generation, &flow)
    }

    /// UI reads still go through `Client::devices`. This is not a second list.
    /// The own-user authority `/keys/query` runs once per generation.
    pub async fn snapshot(&self) -> Result<NativeDeviceSnapshot, &'static str> {
        snapshot_with_gate(
            &self.client,
            self.session_generation,
            Some(&self.own_keys_query),
        )
        .await
    }

    /// Screen and wakeup reads reuse the last homeserver backup answer while
    /// the SDK's local backup state is unchanged (see `ServerBackupCache`).
    pub async fn backup_status(
        &self,
    ) -> Result<crate::app::backup::NativeBackupStatus, &'static str> {
        crate::app::backup::cached_status(
            &self.client,
            self.session_generation,
            &self.server_backup,
        )
        .await
    }

    /// Restore encryption backup. Recovery secret is a method argument only.
    pub async fn restore_backup(
        &self,
        recovery_secret: &str,
    ) -> Result<crate::app::backup::MatrixRestoreBackupResult, &'static str> {
        let result =
            crate::app::backup::restore(&self.client, self.session_generation, recovery_secret)
                .await;
        self.server_backup.invalidate().await;
        result
    }

    pub async fn secret_storage_status(
        &self,
    ) -> Result<crate::app::secret_storage::NativeSecretStorageStatus, &'static str> {
        crate::app::secret_storage::status(&self.client, self.session_generation).await
    }
    pub async fn backup_setup(
        &self,
        passphrase: &str,
    ) -> Result<crate::app::backup::NativeBackupOperationResult, &'static str> {
        let result =
            crate::app::backup::setup(&self.client, self.session_generation, passphrase).await;
        self.server_backup.invalidate().await;
        result
    }
    pub async fn backup_repair(
        &self,
        recovery_secret: &str,
    ) -> Result<crate::app::backup::NativeBackupOperationResult, &'static str> {
        let result =
            crate::app::backup::repair(&self.client, self.session_generation, recovery_secret)
                .await;
        self.server_backup.invalidate().await;
        result
    }
    pub async fn secret_storage_bootstrap(
        &self,
        passphrase: &str,
    ) -> Result<crate::app::secret_storage::SecretStorageSetup, &'static str> {
        let result = crate::app::secret_storage::bootstrap(
            &self.client,
            self.session_generation,
            passphrase,
        )
        .await;
        self.server_backup.invalidate().await;
        result
    }
    pub async fn secret_storage_unlock(
        &self,
        recovery_secret: &str,
    ) -> Result<crate::app::secret_storage::NativeSecretStorageOperationResult, &'static str> {
        let result = crate::app::secret_storage::unlock(
            &self.client,
            self.session_generation,
            recovery_secret,
        )
        .await;
        self.server_backup.invalidate().await;
        result
    }
    pub async fn secret_storage_reset(
        &self,
        passphrase: &str,
    ) -> Result<crate::app::secret_storage::SecretStorageSetup, &'static str> {
        let result =
            crate::app::secret_storage::reset(&self.client, self.session_generation, passphrase)
                .await;
        self.server_backup.invalidate().await;
        result
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
            let selected = snapshot
                .devices
                .iter()
                .find(|device| device.device_id == device_id);
            if device_id.is_empty()
                || device_id == current
                || !selected.is_some_and(super::device_eligible_for_password_logout)
            {
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

/// Homeserver sessions without a matching crypto device: `NoEncryption` only
/// after the crypto store answered. A failed/not-yet-ready store must not
/// paint every session as "Not encrypted" on first load.
fn trust_when_crypto_device_absent(crypto_store_loaded: bool) -> NativeDeviceTrust {
    if crypto_store_loaded {
        project_native_device_trust(NativeDeviceTrustSignals {
            has_crypto_device: false,
            is_dehydrated: false,
            is_verified_with_cross_signing: false,
            is_verified: false,
            is_verified_by_certificate: false,
        })
    } else {
        NativeDeviceTrust::Unverified
    }
}

/// Mirror of the SDK's `has_devices_to_verify_against` predicate over an
/// already-fetched local device set: signed by the owner's cross-signing key,
/// Olm-capable, and not dehydrated. Used when the authority `/keys/query`
/// fails, times out or already ran this generation; after one successful
/// query the SDK keeps this local set current from sync.
fn eligible_local_authority(devices: Option<&UserDevices>) -> bool {
    devices.is_some_and(|devices| {
        devices.devices().any(|device| {
            device.is_cross_signed_by_owner()
                && device.curve25519_key().is_some()
                && !device.is_dehydrated()
        })
    })
}

/// Deliberately categorical: SDK Display/Debug can contain URLs, identifiers or
/// server-provided text and must never be included in verification diagnostics.
fn lookup_failure_category(error: &matrix_sdk::Error) -> &'static str {
    use matrix_sdk::Error;
    match error {
        Error::AuthenticationRequired => "authentication",
        Error::NoOlmMachine => "crypto_not_ready",
        Error::CryptoStoreError(_) | Error::BadCryptoStoreState => "crypto_store",
        Error::CrossProcessLockError(_) => "store_lock",
        Error::Http(error) => http_failure_category(error),
        _ => "other",
    }
}

fn http_failure_category(error: &matrix_sdk::HttpError) -> &'static str {
    use matrix_sdk::HttpError;
    match error {
        HttpError::Reqwest(_) => "transport",
        HttpError::RefreshToken(_) => "refresh_session",
        HttpError::Api(_) => "server_response",
        _ => "http_other",
    }
}

/// Device snapshot that always runs the authority `/keys/query`.
pub async fn snapshot(
    client: &Client,
    session_generation: u64,
) -> Result<NativeDeviceSnapshot, &'static str> {
    snapshot_with_gate(client, session_generation, None).await
}

/// With a gate, the authority `/keys/query` runs only when the gate admits it
/// (once per generation); other reads project the SDK predicate locally.
async fn snapshot_with_gate(
    client: &Client,
    session_generation: u64,
    authority_gate: Option<&OwnKeysQueryGate>,
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
    // The SDK repeats that query on every call while the own identity is
    // missing, and its answer re-sets the verification state, so the owner
    // admits it once per generation.
    let run_authority = authority_gate.is_none_or(OwnKeysQueryGate::try_begin);
    let (eligibility, server_devices, crypto_devices) = tokio::join!(
        async {
            if !run_authority {
                return None;
            }
            Some(
                tokio::time::timeout(
                    Duration::from_secs(8),
                    encryption.has_devices_to_verify_against(),
                )
                .await,
            )
        },
        client.devices(),
        encryption.get_user_devices(user_id),
    );
    if let (Some(gate), true) = (authority_gate, run_authority) {
        gate.finish(matches!(eligibility, Some(Ok(Ok(_)))));
    }
    if std::env::var("SYNARA_VERIFICATION_DIAGNOSTICS").as_deref() == Ok("1") {
        let authority = match &eligibility {
            None => "local",
            Some(Ok(Ok(true))) => "eligible",
            Some(Ok(Ok(false))) => "none",
            Some(Ok(Err(error))) => lookup_failure_category(error),
            Some(Err(_)) => "timeout",
        };
        let server = server_devices
            .as_ref()
            .map(|_| "available")
            .unwrap_or_else(http_failure_category);
        let crypto = crypto_devices
            .as_ref()
            .map(|_| "available")
            .unwrap_or_else(lookup_failure_category);
        eprintln!(
            "synara_verification_snapshot authority={authority} sessions={server} crypto={crypto}"
        );
    }
    let has_devices_to_verify_against = match eligibility {
        Some(Ok(Ok(has_devices))) => Some(has_devices),
        // The authority lookup performs its own `/keys/query`. When it fails,
        // times out or already ran this generation, project the same SDK
        // predicate over the concurrently fetched local device set instead of
        // blanking eligibility to "could not check". A fresh store with no
        // tracked devices projects `Some(false)`, which keeps the actionable
        // "open a verified session" guidance instead of a retry dead-end. If
        // the local set itself failed to load, keep `None` so presenters show
        // Retry rather than a false "no eligible session".
        Some(Ok(Err(_))) | Some(Err(_)) | None => match crypto_devices.as_ref() {
            Ok(devices) => Some(eligible_local_authority(Some(devices))),
            Err(matrix_sdk::Error::NoOlmMachine) | Err(_) => None,
        },
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
    let crypto_store_loaded = crypto_devices.is_ok();
    let crypto_devices = crypto_devices.ok();

    let current_device_id = current_device_id.to_owned();
    let mut devices = server_devices
        .devices
        .into_iter()
        .map(|device| {
            let display_name =
                bounded_optional_hs_text(device.display_name, MAX_DEVICE_DISPLAY_NAME_CHARS);
            let crypto_device = crypto_devices
                .as_ref()
                .and_then(|devices| devices.get(&device.device_id));
            let (trust, is_cross_signed_by_owner, first_seen_ts, ed25519_fingerprint) =
                match crypto_device {
                    Some(crypto_device) => (
                        // Cross-signing is `Verified`. Direct SAS / local trust
                        // without a signature chain is `VerifiedLocallyOnly` so
                        // a completed SAS sheet does not render the peer as
                        // unverified. Missing crypto devices are `NoEncryption`
                        // only when the crypto store itself loaded.
                        project_native_device_trust(NativeDeviceTrustSignals {
                            has_crypto_device: true,
                            is_dehydrated: crypto_device.is_dehydrated(),
                            is_verified_with_cross_signing: crypto_device
                                .is_verified_with_cross_signing(),
                            is_verified: crypto_device.is_verified(),
                            // Own-account session list: X.509 never stands in
                            // for SAS / USK on this device.
                            is_verified_by_certificate: false,
                        }),
                        crypto_device.is_cross_signed_by_owner(),
                        Some(u64::from(crypto_device.first_time_seen_ts().0)),
                        crypto_device
                            .ed25519_key()
                            .and_then(|key| format_ed25519_fingerprint(&key.to_base64())),
                    ),
                    // The creating client uploads via MSC3814 before /keys/query
                    // has the catcher. Keep the SDK default name as Backup device
                    // so Sessions does not offer /devices DELETE on that row.
                    None if display_name.as_deref() == Some("Dehydrated device") => {
                        (NativeDeviceTrust::Dehydrated, false, None, None)
                    }
                    None => (
                        trust_when_crypto_device_absent(crypto_store_loaded),
                        false,
                        None,
                        None,
                    ),
                };
            NativeDeviceSummary {
                is_current: device.device_id == current_device_id,
                device_id: device.device_id.to_string(),
                display_name,
                last_seen_ip: bounded_optional_hs_text(
                    device.last_seen_ip,
                    MAX_DEVICE_LAST_SEEN_IP_CHARS,
                ),
                last_seen_ts: device.last_seen_ts.map(|timestamp| u64::from(timestamp.0)),
                trust,
                is_cross_signed_by_owner,
                first_seen_ts,
                ed25519_fingerprint,
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

    use super::{lookup_failure_category, supported_delete_authentication};

    #[test]
    fn snapshot_diagnostics_classify_without_rendering_sdk_errors() {
        assert_eq!(
            lookup_failure_category(&matrix_sdk::Error::NoOlmMachine),
            "crypto_not_ready"
        );
        assert_eq!(
            lookup_failure_category(&matrix_sdk::Error::BadCryptoStoreState),
            "crypto_store"
        );
        assert_eq!(
            lookup_failure_category(&matrix_sdk::Error::AuthenticationRequired),
            "authentication"
        );
        let error = matrix_sdk::Error::Io(std::io::Error::other("private-server-and-account-data"));
        assert_eq!(lookup_failure_category(&error), "other");
    }

    use crate::app::devices::NativeDeviceDeleteAuthentication;

    #[tokio::test]
    async fn backup_completion_emits_status_update_without_device_change_and_stops_on_drop() {
        use matrix_sdk::{encryption::backups::BackupState, test_utils::mocks::MatrixMockServer};
        use std::{sync::Arc, time::Duration};
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };

        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/_matrix/client/.*/room_keys/version$"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"version":"1"})),
            )
            .expect(1)
            .mount(server.server())
            .await;
        let (send, mut receive) = tokio::sync::mpsc::unbounded_channel();
        let observed = client.clone();
        let owner = super::NativeDeviceOwner::start(
            &client,
            Arc::new(move |signal| {
                let _ = send.send((
                    signal.session_generation,
                    observed.encryption().backups().state(),
                ));
            }),
            42,
        )
        .await
        .unwrap();

        client.encryption().backups().create().await.unwrap();
        assert!(client.encryption().backups().are_enabled().await);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let (generation, state) = receive.recv().await.expect("owner update");
                assert_eq!(generation, 42);
                if state == BackupState::Enabled {
                    break;
                }
            }
        })
        .await
        .expect("SDK backup completion must wake the security UI");

        drop(owner);
        tokio::time::timeout(Duration::from_secs(2), async {
            while receive.recv().await.is_some() {}
        })
        .await
        .expect("dropping the owner must release every status subscription");
    }

    #[tokio::test]
    async fn security_wakeups_ignore_unchanged_sdk_values() {
        use super::{security_wakeups, SecuritySignalState, SecuritySourceUpdate as Update};
        use futures_util::StreamExt;
        use matrix_sdk::encryption::{
            backups::BackupState, recovery::RecoveryState, VerificationState,
        };

        let initial = SecuritySignalState {
            verification: VerificationState::Unverified,
            backup: BackupState::Unknown,
            recovery: RecoveryState::Disabled,
        };
        // What one status read used to cause: the SDK re-sets verification
        // after its `/keys/query` and recovery after the account-data GET,
        // and `subscribe_reset` replays the current values.
        let echo = [
            Update::Verification(VerificationState::Unverified),
            Update::Recovery(RecoveryState::Disabled),
            Update::Backup(Some(BackupState::Unknown)),
            Update::Devices {
                own_user_changed: false,
            },
        ];
        let echoes = futures_util::stream::iter(echo.into_iter().cycle().take(400));
        assert_eq!(
            security_wakeups(echoes, initial).count().await,
            0,
            "unchanged values must not wake the shells"
        );

        let transitions = futures_util::stream::iter([
            Update::Verification(VerificationState::Unverified),
            Update::Verification(VerificationState::Verified),
            Update::Verification(VerificationState::Verified),
            Update::Recovery(RecoveryState::Enabled),
            Update::Recovery(RecoveryState::Enabled),
            Update::Backup(Some(BackupState::Enabling)),
            Update::Backup(Some(BackupState::Enabled)),
            Update::Backup(Some(BackupState::Enabled)),
            Update::Devices {
                own_user_changed: true,
            },
            Update::Devices {
                own_user_changed: false,
            },
            // A lagged backup broadcast hides the value; refresh once.
            Update::Backup(None),
        ]);
        assert_eq!(
            security_wakeups(transitions, initial).count().await,
            6,
            "verification, recovery, two backup transitions, own devices, lag"
        );
    }

    #[tokio::test]
    async fn owner_start_does_not_echo_the_current_sdk_state() {
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        use std::{sync::Arc, time::Duration};

        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        // Let the SDK's own start-up transitions (verification leaves
        // `Unknown`) happen first; those are real changes.
        client
            .encryption()
            .wait_for_e2ee_initialization_tasks()
            .await;
        let mut verification = client.encryption().verification_state();
        tokio::time::timeout(Duration::from_secs(2), async {
            while verification.get() == matrix_sdk::encryption::VerificationState::Unknown {
                verification.next().await;
            }
        })
        .await
        .expect("the SDK settles its verification state");
        let (send, mut receive) = tokio::sync::mpsc::unbounded_channel();
        let observed = client.clone();
        let before = (
            client.encryption().verification_state().get(),
            client.encryption().backups().state(),
            client.encryption().recovery().state(),
        );
        let _owner = super::NativeDeviceOwner::start(
            &client,
            Arc::new(move |_| {
                let _ = send.send((
                    observed.encryption().verification_state().get(),
                    observed.encryption().backups().state(),
                    observed.encryption().recovery().state(),
                ));
            }),
            7,
        )
        .await
        .unwrap();
        // Verification and recovery use `subscribe_reset`, which yields the
        // current value at once. That replay is not a change.
        let woke = tokio::time::timeout(Duration::from_millis(300), receive.recv()).await;
        assert!(
            woke.is_err(),
            "starting the owner must not wake the shells: {before:?} -> {woke:?}"
        );
    }

    #[tokio::test]
    async fn owner_snapshots_send_the_authority_keys_query_once_per_generation() {
        use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
        use std::sync::Arc;
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };

        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
            .build()
            .await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/_matrix/client/.*/keys/query$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(server.server())
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/_matrix/client/.*/devices$"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"devices": []})),
            )
            .mount(server.server())
            .await;
        let keys_queries = || async {
            server
                .server()
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|request| request.url.path().ends_with("/keys/query"))
                .count()
        };

        let owner = super::NativeDeviceOwner::start(&client, Arc::new(|_| {}), 3)
            .await
            .unwrap();
        // This account has no cross-signing identity, the case in which the
        // SDK repeats its own-user query on every eligibility check.
        let first = owner.snapshot().await.unwrap();
        assert_eq!(first.has_devices_to_verify_against, Some(false));
        let after_first = keys_queries().await;
        assert!(after_first >= 1, "the first read asks the homeserver");
        for _ in 0..3 {
            let snapshot = owner.snapshot().await.unwrap();
            assert_eq!(snapshot.has_devices_to_verify_against, Some(false));
        }
        assert_eq!(
            keys_queries().await,
            after_first,
            "later reads project eligibility from the local store"
        );

        // The ungated function keeps the authoritative behaviour.
        super::snapshot(&client, 3).await.unwrap();
        assert!(keys_queries().await > after_first);
    }

    #[test]
    fn own_keys_query_gate_admits_one_query_and_re_arms_after_failure() {
        let gate = super::OwnKeysQueryGate::default();
        assert!(gate.try_begin());
        assert!(!gate.try_begin(), "a concurrent read uses the local store");
        gate.finish(false);
        assert!(gate.try_begin(), "a failed query is retried");
        gate.finish(true);
        assert!(!gate.try_begin());
    }

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
    fn missing_crypto_device_does_not_claim_no_encryption_until_the_store_answers() {
        use crate::app::devices::NativeDeviceTrust;
        assert_eq!(
            super::trust_when_crypto_device_absent(false),
            NativeDeviceTrust::Unverified
        );
        assert_eq!(
            super::trust_when_crypto_device_absent(true),
            NativeDeviceTrust::NoEncryption
        );
    }

    #[test]
    fn trust_signals_are_sampled_from_one_crypto_device_handle() {
        let source = include_str!("live.rs");
        let snapshot = source
            .split("pub async fn snapshot(\n")
            .nth(1)
            .and_then(|rest| rest.split("pub fn supported_delete_authentication").next())
            .expect("device snapshot");
        assert!(snapshot.contains("is_verified_with_cross_signing()"));
        assert!(snapshot.contains(".is_verified()"));
        assert!(
            snapshot.contains("trust_when_crypto_device_absent(crypto_store_loaded)"),
            "a failed crypto fetch must not collapse every homeserver session to NoEncryption"
        );
    }
}
