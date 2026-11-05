//! The account's draft-sync key: secret storage, the local cache, and peer
//! sharing between the user's own verified devices.
//!
//! Lifecycle (see `docs/design/cross-device-drafts.md`):
//! - Any device that opens secret storage reads `in.synara.drafts.v1`, or
//!   stores this device's cached key (or a new one) there when it is missing.
//!   A recovery-key reset therefore carries the key into the new store.
//! - An identity reset forgets the cached key, so the next secret-storage
//!   enrolment generates a fresh one.
//! - A device that never opens secret storage asks the user's other
//!   cross-signing-verified devices over Olm; only a verified own device that
//!   we asked is believed, and only a verified own device is answered.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use matrix_sdk::{
    deserialized_responses::{EncryptionInfo, VerificationState},
    encryption::secret_storage::SecretStore,
    ruma::{
        events::{secret::request::SecretName, AnyToDeviceEvent, AnyToDeviceEventContent},
        serde::Raw,
        OwnedDeviceId,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use zeroize::Zeroizing;

use super::DraftsKey;

pub const DRAFTS_SECRET_NAME: &str = "in.synara.drafts.v1";
pub const DRAFTS_KEY_CACHE_KEY: &[u8] = b"in.synara.drafts.key.v1";
pub const DRAFTS_KEY_REQUEST_EVENT_TYPE: &str = "in.synara.drafts.key.request";
pub const DRAFTS_KEY_SHARE_EVENT_TYPE: &str = "in.synara.drafts.key.share";
const MAX_OUTSTANDING_REQUESTS: usize = 8;
const MAX_REQUEST_ID_BYTES: usize = 64;

/// Bumped whenever this process learns, replaces or forgets the key, so the
/// live owner can tell shells to re-read drafts.
static KEY_EPOCH: LazyLock<watch::Sender<u64>> = LazyLock::new(|| watch::channel(0).0);

pub(super) fn subscribe_key_epoch() -> watch::Receiver<u64> {
    KEY_EPOCH.subscribe()
}

fn bump_key_epoch() {
    KEY_EPOCH.send_modify(|epoch| *epoch = epoch.wrapping_add(1));
}

fn secret_name() -> SecretName {
    SecretName::from(DRAFTS_SECRET_NAME)
}

pub(super) async fn load_cached_key(client: &Client) -> Option<DraftsKey> {
    let bytes = Zeroizing::new(
        client
            .state_store()
            .get_custom_value(DRAFTS_KEY_CACHE_KEY)
            .await
            .ok()??,
    );
    DraftsKey::from_bytes(&bytes).ok()
}

async fn store_cached_key(client: &Client, key: &DraftsKey) -> Result<(), &'static str> {
    if load_cached_key(client).await.as_ref() == Some(key) {
        return Ok(());
    }
    client
        .state_store()
        .set_custom_value_no_read(DRAFTS_KEY_CACHE_KEY, key.as_bytes().to_vec())
        .await
        .map_err(|_| "drafts-key-cache-write-failed")?;
    bump_key_epoch();
    Ok(())
}

pub(super) async fn forget_cached_key(client: &Client) {
    let _ = client
        .state_store()
        .remove_custom_value(DRAFTS_KEY_CACHE_KEY)
        .await;
    bump_key_epoch();
}

/// Read the key from an open secret store, or store ours there.
pub(super) async fn sync_with_secret_store(
    client: &Client,
    store: &SecretStore,
) -> Result<DraftsKey, &'static str> {
    if let Some(secret) = store
        .get_secret(secret_name())
        .await
        .map_err(|_| "drafts-key-secret-read-failed")?
    {
        let secret = Zeroizing::new(secret);
        let key = DraftsKey::from_base64(&secret)?;
        store_cached_key(client, &key).await?;
        return Ok(key);
    }
    let key = match load_cached_key(client).await {
        Some(key) => key,
        None => DraftsKey::generate(),
    };
    store
        .put_secret(secret_name(), &key.to_base64())
        .await
        .map_err(|_| "drafts-key-secret-write-failed")?;
    // Two devices may enrol at once: whichever write landed is the key.
    let stored = store
        .get_secret(secret_name())
        .await
        .map_err(|_| "drafts-key-secret-read-failed")?
        .ok_or("drafts-key-secret-readback-missing")?;
    let stored = DraftsKey::from_base64(&Zeroizing::new(stored))?;
    store_cached_key(client, &stored).await?;
    Ok(stored)
}

/// Called with the recovery secret right after the SDK opened (enrolled,
/// unlocked or re-keyed) secret storage. Failures never fail recovery.
pub(crate) async fn on_secret_store_open(client: &Client, secret: &str) {
    if secret.is_empty() {
        return;
    }
    let Ok(store) = client
        .encryption()
        .secret_storage()
        .open_secret_store(secret)
        .await
    else {
        return;
    };
    let _ = sync_with_secret_store(client, &store).await;
}

/// A new cryptographic identity: drafts sealed under the old key stay sealed.
pub(crate) async fn on_identity_reset(client: &Client) {
    forget_cached_key(client).await;
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyRequestContent {
    request_id: String,
    requesting_device_id: String,
}

#[derive(Serialize, Deserialize)]
struct KeyShareContent {
    request_id: String,
    key: String,
}

#[derive(Deserialize)]
struct RawToDevice<T> {
    #[serde(rename = "type")]
    event_type: String,
    content: T,
}

/// Request ids this device sent and has not had answered.
#[derive(Default)]
pub(super) struct PeerShareState {
    outstanding: Mutex<HashSet<String>>,
}

impl PeerShareState {
    fn remember(&self, request_id: &str) {
        let mut outstanding = self.outstanding.lock().unwrap_or_else(|e| e.into_inner());
        if outstanding.len() >= MAX_OUTSTANDING_REQUESTS {
            outstanding.clear();
        }
        outstanding.insert(request_id.to_owned());
    }

    fn take(&self, request_id: &str) -> bool {
        self.outstanding
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(request_id)
    }
}

fn random_request_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("request id entropy");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The user's other devices that cross-signing says are theirs.
async fn verified_own_devices(client: &Client) -> Vec<matrix_sdk::encryption::identities::Device> {
    let (Some(user_id), Some(own_device)) = (client.user_id(), client.device_id()) else {
        return Vec::new();
    };
    let Ok(devices) = client.encryption().get_user_devices(user_id).await else {
        return Vec::new();
    };
    devices
        .devices()
        .filter(|device| device.device_id() != own_device)
        .filter(|device| !device.is_deleted() && device.is_cross_signed_by_owner())
        .collect()
}

/// Ask the user's verified devices for the key. Safe to call repeatedly.
pub(super) async fn request_from_peers(
    client: &Client,
    state: &PeerShareState,
) -> Result<usize, &'static str> {
    let own_device = client.device_id().ok_or("drafts-key-request-no-device")?;
    let devices = verified_own_devices(client).await;
    if devices.is_empty() {
        return Ok(0);
    }
    let request_id = random_request_id();
    let content = Raw::new(&KeyRequestContent {
        request_id: request_id.clone(),
        requesting_device_id: own_device.to_string(),
    })
    .map_err(|_| "drafts-key-request-serialize-failed")?
    .cast_unchecked::<AnyToDeviceEventContent>();
    state.remember(&request_id);
    let failures = client
        .encryption()
        .encrypt_and_send_raw_to_device(
            devices.iter().collect(),
            DRAFTS_KEY_REQUEST_EVENT_TYPE,
            content,
            matrix_sdk_crypto::CollectStrategy::AllDevices,
        )
        .await
        .map_err(|_| "drafts-key-request-send-failed")?;
    Ok(devices.len().saturating_sub(failures.len()))
}

/// Olm-decrypted, sent by our own account from a device cross-signing
/// vouches for. Anything else is ignored.
async fn verified_own_sender(
    client: &Client,
    encryption_info: Option<&EncryptionInfo>,
) -> Option<OwnedDeviceId> {
    let info = encryption_info?;
    if Some(info.sender.as_ref()) != client.user_id()
        || info.verification_state != VerificationState::Verified
    {
        return None;
    }
    let device_id = info.sender_device.clone()?;
    if Some(device_id.as_ref()) == client.device_id() {
        return None;
    }
    let device = client
        .encryption()
        .get_device(&info.sender, &device_id)
        .await
        .ok()??;
    (!device.is_deleted() && device.is_cross_signed_by_owner()).then_some(device_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PeerEventOutcome {
    Ignored,
    Answered,
    KeyReceived,
}

pub(super) async fn handle_to_device(
    client: &Client,
    state: &PeerShareState,
    raw: &Raw<AnyToDeviceEvent>,
    encryption_info: Option<&EncryptionInfo>,
) -> PeerEventOutcome {
    let Ok(event_type) = raw.get_field::<String>("type") else {
        return PeerEventOutcome::Ignored;
    };
    match event_type.as_deref() {
        Some(DRAFTS_KEY_REQUEST_EVENT_TYPE) => answer_request(client, raw, encryption_info).await,
        Some(DRAFTS_KEY_SHARE_EVENT_TYPE) => {
            accept_share(client, state, raw, encryption_info).await
        }
        _ => PeerEventOutcome::Ignored,
    }
}

async fn answer_request(
    client: &Client,
    raw: &Raw<AnyToDeviceEvent>,
    encryption_info: Option<&EncryptionInfo>,
) -> PeerEventOutcome {
    let Some(sender_device) = verified_own_sender(client, encryption_info).await else {
        return PeerEventOutcome::Ignored;
    };
    let Ok(event) = raw.deserialize_as_unchecked::<RawToDevice<KeyRequestContent>>() else {
        return PeerEventOutcome::Ignored;
    };
    if event.event_type != DRAFTS_KEY_REQUEST_EVENT_TYPE
        || event.content.requesting_device_id != sender_device.as_str()
        || event.content.request_id.is_empty()
        || event.content.request_id.len() > MAX_REQUEST_ID_BYTES
    {
        return PeerEventOutcome::Ignored;
    }
    let Some(key) = load_cached_key(client).await else {
        return PeerEventOutcome::Ignored;
    };
    let Some(user_id) = client.user_id() else {
        return PeerEventOutcome::Ignored;
    };
    let Ok(Some(device)) = client
        .encryption()
        .get_device(user_id, &sender_device)
        .await
    else {
        return PeerEventOutcome::Ignored;
    };
    let share = KeyShareContent {
        request_id: event.content.request_id,
        key: key.to_base64().to_string(),
    };
    let Ok(content) = Raw::new(&share) else {
        return PeerEventOutcome::Ignored;
    };
    let sent = client
        .encryption()
        .encrypt_and_send_raw_to_device(
            vec![&device],
            DRAFTS_KEY_SHARE_EVENT_TYPE,
            content.cast_unchecked::<AnyToDeviceEventContent>(),
            matrix_sdk_crypto::CollectStrategy::AllDevices,
        )
        .await;
    drop(share);
    match sent {
        Ok(failures) if failures.is_empty() => PeerEventOutcome::Answered,
        _ => PeerEventOutcome::Ignored,
    }
}

async fn accept_share(
    client: &Client,
    state: &PeerShareState,
    raw: &Raw<AnyToDeviceEvent>,
    encryption_info: Option<&EncryptionInfo>,
) -> PeerEventOutcome {
    if verified_own_sender(client, encryption_info).await.is_none() {
        return PeerEventOutcome::Ignored;
    }
    let Ok(event) = raw.deserialize_as_unchecked::<RawToDevice<KeyShareContent>>() else {
        return PeerEventOutcome::Ignored;
    };
    let key_text = Zeroizing::new(event.content.key);
    // Only answers to our own requests: an unsolicited key is never adopted.
    if event.event_type != DRAFTS_KEY_SHARE_EVENT_TYPE || !state.take(&event.content.request_id) {
        return PeerEventOutcome::Ignored;
    }
    let Ok(key) = DraftsKey::from_base64(&key_text) else {
        return PeerEventOutcome::Ignored;
    };
    match store_cached_key(client, &key).await {
        Ok(()) => PeerEventOutcome::KeyReceived,
        Err(_) => PeerEventOutcome::Ignored,
    }
}
