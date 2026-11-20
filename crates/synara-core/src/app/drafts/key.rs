//! The account's draft-sync key: secret storage, the local cache, and peer
//! sharing between the user's own verified devices.
//!
//! Lifecycle (see `docs/design/cross-device-drafts.md`):
//! - A cross-signing-verified device without a key asks the user's other
//!   verified devices over Olm. When none of them answers in time (or there
//!   are none), it creates the key itself: the recovery key is not needed.
//! - Devices may so create different keys. Every merge point (a peer's share,
//!   secret storage) applies one total order, [`KeyRank`], so they converge:
//!   the higher generation wins, then the lower key id.
//! - Only a verified own device that we asked is believed, and only a
//!   verified own device is answered.
//! - Any device that opens secret storage merges `in.synara.drafts.v1` with
//!   its cached key and stores the winner there, so a recovery-key reset
//!   carries the key into the new store.
//! - An identity reset forgets the cached key and raises the generation
//!   floor, so the old key can never be adopted again on that device and the
//!   key it creates next wins on the others.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

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

use super::{DraftsKey, DRAFTS_KEY_BYTES};

pub const DRAFTS_SECRET_NAME: &str = "in.synara.drafts.v1";
pub const DRAFTS_KEY_CACHE_KEY: &[u8] = b"in.synara.drafts.key.v1";
/// Lowest generation this device accepts (raised by an identity reset).
pub const DRAFTS_KEY_FLOOR_CACHE_KEY: &[u8] = b"in.synara.drafts.key_floor.v1";
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

/// Where a key stands in the one total order all devices share: the higher
/// generation wins, then the lower key id. It holds nothing secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeyRank {
    pub generation: u32,
    pub key_id: String,
}

impl KeyRank {
    pub(super) fn beats(&self, other: &Self) -> bool {
        match self.generation.cmp(&other.generation) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => self.key_id < other.key_id,
        }
    }
}

/// A drafts key and its generation. Generation 0 is every key made before an
/// identity reset; each reset makes the next key one generation higher.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HeldKey {
    pub key: DraftsKey,
    pub generation: u32,
}

impl HeldKey {
    pub(super) fn rank(&self) -> KeyRank {
        KeyRank {
            generation: self.generation,
            key_id: self.key.key_id(),
        }
    }

    /// The one of two keys every device keeps.
    pub(super) fn preferred(self, other: Self) -> Self {
        if other.rank().beats(&self.rank()) {
            other
        } else {
            self
        }
    }

    /// Local cache form: the key, then the generation when it is not 0 (so a
    /// generation-0 cache reads the same as before generations existed).
    fn to_cache_bytes(&self) -> Zeroizing<Vec<u8>> {
        let mut bytes = Zeroizing::new(self.key.as_bytes().to_vec());
        if self.generation > 0 {
            bytes.extend_from_slice(&self.generation.to_le_bytes());
        }
        bytes
    }

    fn from_cache_bytes(bytes: &[u8]) -> Option<Self> {
        let (key, generation) = match bytes.len() {
            DRAFTS_KEY_BYTES => (bytes, 0),
            len if len == DRAFTS_KEY_BYTES + 4 => {
                let (key, generation) = bytes.split_at(DRAFTS_KEY_BYTES);
                (key, u32::from_le_bytes(generation.try_into().ok()?))
            }
            _ => return None,
        };
        Some(Self {
            key: DraftsKey::from_bytes(key).ok()?,
            generation,
        })
    }

    /// Secret-storage form: the base64 key, then `.<generation>` when it is
    /// not 0 (base64 never contains a dot).
    pub(super) fn to_secret(&self) -> Zeroizing<String> {
        let mut secret = self.key.to_base64();
        if self.generation > 0 {
            secret.push('.');
            secret.push_str(&self.generation.to_string());
        }
        secret
    }

    pub(super) fn from_secret(secret: &str) -> Result<Self, &'static str> {
        let secret = secret.trim();
        let (key, generation) = match secret.split_once('.') {
            Some((key, generation)) => (
                key,
                generation
                    .parse::<u32>()
                    .map_err(|_| "drafts-key-invalid-generation")?,
            ),
            None => (secret, 0),
        };
        Ok(Self {
            key: DraftsKey::from_base64(key)?,
            generation,
        })
    }
}

pub(super) async fn load_held_key(client: &Client) -> Option<HeldKey> {
    let bytes = Zeroizing::new(
        client
            .state_store()
            .get_custom_value(DRAFTS_KEY_CACHE_KEY)
            .await
            .ok()??,
    );
    HeldKey::from_cache_bytes(&bytes)
}

pub(super) async fn load_cached_key(client: &Client) -> Option<DraftsKey> {
    load_held_key(client).await.map(|held| held.key)
}

async fn store_held_key(client: &Client, held: &HeldKey) -> Result<(), &'static str> {
    if load_held_key(client).await.as_ref() == Some(held) {
        return Ok(());
    }
    client
        .state_store()
        .set_custom_value_no_read(DRAFTS_KEY_CACHE_KEY, held.to_cache_bytes().to_vec())
        .await
        .map_err(|_| "drafts-key-cache-write-failed")?;
    bump_key_epoch();
    Ok(())
}

pub(super) async fn load_floor(client: &Client) -> u32 {
    client
        .state_store()
        .get_custom_value(DRAFTS_KEY_FLOOR_CACHE_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|bytes| Some(u32::from_le_bytes(bytes.as_slice().try_into().ok()?)))
        .unwrap_or(0)
}

/// Forget the key and refuse its generation from now on.
pub(super) async fn forget_cached_key(client: &Client) {
    let floor = load_floor(client).await;
    let retired = load_held_key(client)
        .await
        .map_or(floor, |held| held.generation.max(floor).saturating_add(1));
    let _ = client
        .state_store()
        .set_custom_value_no_read(DRAFTS_KEY_FLOOR_CACHE_KEY, retired.to_le_bytes().to_vec())
        .await;
    let _ = client
        .state_store()
        .remove_custom_value(DRAFTS_KEY_CACHE_KEY)
        .await;
    bump_key_epoch();
}

/// Whether cross-signing vouches for this device: only then can it exchange
/// the key with the user's other devices.
pub(super) async fn own_device_verified(client: &Client) -> bool {
    matches!(
        client.encryption().get_own_device().await,
        Ok(Some(device)) if device.is_verified_with_cross_signing()
    )
}

/// Create the key on this verified device when it still has none. Peers that
/// created their own converge with it through [`KeyRank`].
pub(super) async fn create_key_if_verified(client: &Client) -> Option<HeldKey> {
    if let Some(held) = load_held_key(client).await {
        return Some(held);
    }
    if !own_device_verified(client).await {
        return None;
    }
    let held = HeldKey {
        key: DraftsKey::generate(),
        generation: load_floor(client).await,
    };
    store_held_key(client, &held).await.ok()?;
    Some(held)
}

/// Merge an open secret store with this device's key: keep the one that
/// wins, and store it there when the store holds another (or none).
pub(super) async fn sync_with_secret_store(
    client: &Client,
    store: &SecretStore,
) -> Result<DraftsKey, &'static str> {
    let floor = load_floor(client).await;
    let read_stored = || async {
        let Some(secret) = store
            .get_secret(secret_name())
            .await
            .map_err(|_| "drafts-key-secret-read-failed")?
        else {
            return Ok::<_, &'static str>(None);
        };
        let held = HeldKey::from_secret(&Zeroizing::new(secret))?;
        // A key from before this device's identity reset reads as absent.
        Ok((held.generation >= floor).then_some(held))
    };
    let stored = read_stored().await?;
    let cached = load_held_key(client).await;
    let mut winner = match (stored.clone(), cached) {
        (Some(stored), Some(cached)) => stored.preferred(cached),
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => HeldKey {
            key: DraftsKey::generate(),
            generation: floor,
        },
    };
    if stored.as_ref() != Some(&winner) {
        store
            .put_secret(secret_name(), &winner.to_secret())
            .await
            .map_err(|_| "drafts-key-secret-write-failed")?;
        // Two devices may enrol at once: read back what landed.
        if let Some(landed) = read_stored().await? {
            winner = winner.preferred(landed);
        }
    }
    store_held_key(client, &winner).await?;
    Ok(winner.key)
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
    /// The key the requester already holds, if any, so a peer answers only
    /// when its own key wins. Absent from clients that never ask once they
    /// hold a key (Synara before 3.3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    generation: u32,
}

#[derive(Serialize, Deserialize)]
struct KeyShareContent {
    request_id: String,
    key: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    generation: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Deserialize)]
struct RawToDevice<T> {
    #[serde(rename = "type")]
    event_type: String,
    content: T,
}

/// Request ids this device sent, and what its last request announced.
#[derive(Default)]
pub(super) struct PeerShareState {
    outstanding: Mutex<HashSet<String>>,
    last_request: Mutex<Option<(Instant, Option<KeyRank>)>>,
    /// The winning key this device last asked a peer back for.
    last_ask_back: Mutex<Option<(Instant, KeyRank)>>,
}

impl PeerShareState {
    fn remember(&self, request_id: &str) {
        let mut outstanding = self.outstanding.lock().unwrap_or_else(|e| e.into_inner());
        if outstanding.len() >= MAX_OUTSTANDING_REQUESTS {
            outstanding.clear();
        }
        outstanding.insert(request_id.to_owned());
    }

    /// Several peers may answer one request, so an id stays valid until it
    /// is crowded out.
    fn asked(&self, request_id: &str) -> bool {
        self.outstanding
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(request_id)
    }

    /// Whether a request announcing `rank` is due: always when the held key
    /// changed since the last one, otherwise once `repeat_after` has passed.
    fn begin_request(&self, rank: Option<&KeyRank>, repeat_after: Option<Duration>) -> bool {
        let mut last = self.last_request.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, announced)) = last.as_ref() {
            let unchanged = announced.as_ref() == rank;
            let fresh = repeat_after.is_none_or(|after| at.elapsed() < after);
            if unchanged && fresh {
                return false;
            }
        }
        *last = Some((Instant::now(), rank.cloned()));
        true
    }

    /// Whether to ask back for a peer's winning key now: at once for a key
    /// we have not asked for, so a second, better key announced right after
    /// another is not missed; otherwise once per `repeat_after`.
    pub(super) fn begin_ask_back(&self, wanted: &KeyRank, repeat_after: Duration) -> bool {
        let mut last = self.last_ask_back.lock().unwrap_or_else(|e| e.into_inner());
        if last
            .as_ref()
            .is_some_and(|(at, asked)| asked == wanted && at.elapsed() < repeat_after)
        {
            return false;
        }
        *last = Some((Instant::now(), wanted.clone()));
        true
    }

    fn forget_request(&self) {
        *self.last_request.lock().unwrap_or_else(|e| e.into_inner()) = None;
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

/// What a call to [`request_from_peers`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PeerRequest {
    /// Sent to this many verified devices.
    Sent(usize),
    /// A request announcing the same key is recent enough.
    Throttled,
    /// The user has no other verified device.
    NoPeers,
}

/// Tell the user's verified devices which key this device holds (or that it
/// holds none). A device whose key wins answers with it; one whose key loses
/// asks back. `repeat_after: None` sends only when the held key changed since
/// the last request.
pub(super) async fn request_from_peers(
    client: &Client,
    state: &PeerShareState,
    repeat_after: Option<Duration>,
) -> Result<PeerRequest, &'static str> {
    let own_device = client.device_id().ok_or("drafts-key-request-no-device")?;
    let rank = load_held_key(client).await.map(|held| held.rank());
    if !state.begin_request(rank.as_ref(), repeat_after) {
        return Ok(PeerRequest::Throttled);
    }
    let devices = verified_own_devices(client).await;
    if devices.is_empty() {
        // Nobody to ask: allow an immediate retry once a device appears.
        state.forget_request();
        return Ok(PeerRequest::NoPeers);
    }
    let request_id = random_request_id();
    let content = Raw::new(&KeyRequestContent {
        request_id: request_id.clone(),
        requesting_device_id: own_device.to_string(),
        key_id: rank.as_ref().map(|rank| rank.key_id.clone()),
        generation: rank.as_ref().map_or(0, |rank| rank.generation),
    })
    .map_err(|_| "drafts-key-request-serialize-failed")?
    .cast_unchecked::<AnyToDeviceEventContent>();
    state.remember(&request_id);
    let sent = client
        .encryption()
        .encrypt_and_send_raw_to_device(
            devices.iter().collect(),
            DRAFTS_KEY_REQUEST_EVENT_TYPE,
            content,
            matrix_sdk_crypto::CollectStrategy::AllDevices,
        )
        .await;
    match sent {
        Ok(failures) if failures.len() < devices.len() => {
            Ok(PeerRequest::Sent(devices.len() - failures.len()))
        }
        Ok(_) => {
            state.forget_request();
            Ok(PeerRequest::Sent(0))
        }
        Err(_) => {
            state.forget_request();
            Err("drafts-key-request-send-failed")
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PeerEventOutcome {
    Ignored,
    Answered,
    KeyReceived,
    /// A peer's request announced this key, which wins over ours: send our
    /// own request so the peer answers with it.
    AskBackFor(KeyRank),
    /// A peer shared a key that loses to ours: announce ours (unless we just
    /// did) so the peer asks for it.
    Announce,
}

/// What to do with a verified peer's request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestDecision {
    Ignore,
    Answer,
    AskBack,
}

pub(super) fn decide_request(mine: Option<&KeyRank>, theirs: Option<&KeyRank>) -> RequestDecision {
    match (mine, theirs) {
        // Keyless devices each send their own request.
        (None, _) => RequestDecision::Ignore,
        (Some(_), None) => RequestDecision::Answer,
        (Some(mine), Some(theirs)) if mine == theirs => RequestDecision::Ignore,
        (Some(mine), Some(theirs)) if mine.beats(theirs) => RequestDecision::Answer,
        (Some(_), Some(_)) => RequestDecision::AskBack,
    }
}

/// What to do with a key a verified peer sent in answer to our request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShareDecision {
    Ignore,
    Adopt,
    AskBack,
}

pub(super) fn decide_share(mine: Option<&KeyRank>, theirs: &KeyRank, floor: u32) -> ShareDecision {
    if theirs.generation < floor {
        // Retired by this device's identity reset. Ours, once made, wins.
        return if mine.is_some() {
            ShareDecision::AskBack
        } else {
            ShareDecision::Ignore
        };
    }
    match mine {
        None => ShareDecision::Adopt,
        Some(mine) if mine == theirs => ShareDecision::Ignore,
        Some(mine) if theirs.beats(mine) => ShareDecision::Adopt,
        Some(_) => ShareDecision::AskBack,
    }
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
    let held = load_held_key(client).await;
    let theirs = event.content.key_id.map(|key_id| KeyRank {
        generation: event.content.generation,
        key_id,
    });
    let held = match (
        decide_request(held.as_ref().map(HeldKey::rank).as_ref(), theirs.as_ref()),
        held,
    ) {
        (RequestDecision::Answer, Some(held)) => held,
        (RequestDecision::AskBack, _) => {
            return theirs.map_or(PeerEventOutcome::Ignored, PeerEventOutcome::AskBackFor)
        }
        _ => return PeerEventOutcome::Ignored,
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
        key: held.key.to_base64().to_string(),
        generation: held.generation,
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
    if event.event_type != DRAFTS_KEY_SHARE_EVENT_TYPE || !state.asked(&event.content.request_id) {
        return PeerEventOutcome::Ignored;
    }
    let Ok(key) = DraftsKey::from_base64(&key_text) else {
        return PeerEventOutcome::Ignored;
    };
    let received = HeldKey {
        key,
        generation: event.content.generation,
    };
    let mine = load_held_key(client).await.map(|held| held.rank());
    match decide_share(mine.as_ref(), &received.rank(), load_floor(client).await) {
        ShareDecision::Ignore => PeerEventOutcome::Ignored,
        ShareDecision::AskBack => PeerEventOutcome::Announce,
        ShareDecision::Adopt => match store_held_key(client, &received).await {
            Ok(()) => PeerEventOutcome::KeyReceived,
            Err(_) => PeerEventOutcome::Ignored,
        },
    }
}

#[cfg(test)]
#[path = "key_tests.rs"]
mod tests;
