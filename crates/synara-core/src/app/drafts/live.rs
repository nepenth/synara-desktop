//! Live `in.synara.draft` owner: debounced sealed writes, merged reads, sync
//! wakeups and the peer key exchange. Shells keep their local drafts as the
//! composer's source of truth; this layers sync on top.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use matrix_sdk::{
    deserialized_responses::EncryptionInfo,
    event_handler::{EventHandlerDropGuard, RawEvent},
    ruma::{
        api::{client::config::get_room_account_data, error::ErrorKind},
        events::{
            AnyRoomAccountDataEvent, AnyRoomAccountDataEventContent, AnyToDeviceEvent,
            RoomAccountDataEventType,
        },
        serde::Raw,
        OwnedRoomId, RoomId,
    },
    Client, Room,
};
use serde::{Deserialize, Serialize};
use serde_json::value::{to_raw_value, RawValue as RawJsonValue};
use tokio::sync::Mutex as AsyncMutex;

use super::key::{self, PeerShareState};
use super::{
    cap_payload, decide_remote, merge_payloads, next_updated_at, open_envelope, parse_envelope,
    prune_tombstones, seal_payload, validate_slot_key, DraftDecision, DraftsKey, LocalDraftState,
    SynaraDraftSlot, SynaraDraftsPayload, DRAFTS_EVENT_TYPE, DRAFTS_PAYLOAD_VERSION,
    MAX_DRAFT_EVENT_ID_BYTES,
};

/// Typing pause before a write.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(1_000);
/// Longest a continuously edited draft waits before it is written anyway.
const SAVE_MAX_DELAY: Duration = Duration::from_millis(5_000);
const PEER_REQUEST_INTERVAL: Duration = Duration::from_secs(60);
const MAX_MARKDOWN_BYTES: usize = 256 * 1024;
const MAX_SLATE_BYTES: usize = 512 * 1024;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

fn drafts_type() -> RoomAccountDataEventType {
    RoomAccountDataEventType::from(DRAFTS_EVENT_TYPE)
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeDraftKeyState {
    /// Drafts sync on this device.
    Ready,
    /// Asked the user's other verified devices; waiting for an answer.
    Requesting,
    /// No key and no device to ask: the recovery key unlocks draft sync.
    NeedsRecovery,
    /// The setting is off on this device.
    Disabled,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDraftSyncStatus {
    pub session_generation: u64,
    pub enabled: bool,
    pub key_state: NativeDraftKeyState,
}

/// A composer's draft as one device last synced it.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRemoteDraft {
    pub room_id: String,
    pub slot: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub updated_at: u64,
    pub device_id: String,
    pub cleared: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional, type = "unknown"))]
    pub desktop_slate: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub edit_of: Option<String>,
    pub truncated: bool,
}

impl NativeRemoteDraft {
    fn from_slot(room_id: &str, slot_key: &str, slot: SynaraDraftSlot) -> Self {
        Self {
            room_id: room_id.to_owned(),
            slot: slot_key.to_owned(),
            updated_at: slot.updated_at,
            device_id: slot.device_id,
            cleared: slot.cleared,
            markdown: slot.markdown,
            desktop_slate: slot.desktop_slate,
            reply_to: slot.reply_to,
            edit_of: slot.edit_of,
            truncated: slot.truncated,
        }
    }
}

/// What the shell's composer holds right now.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeLocalDraftState {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional, type = "number"))]
    pub updated_at: Option<u64>,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub markdown: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeDraftDecision {
    Keep,
    Apply,
    Prompt,
}

impl From<DraftDecision> for NativeDraftDecision {
    fn from(decision: DraftDecision) -> Self {
        match decision {
            DraftDecision::Keep => Self::Keep,
            DraftDecision::Apply => Self::Apply,
            DraftDecision::Prompt => Self::Prompt,
        }
    }
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDraftResolution {
    pub session_generation: u64,
    pub decision: NativeDraftDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub draft: Option<NativeRemoteDraft>,
}

/// A room's synced drafts changed (`room_id: None`: re-read every composer,
/// e.g. the key arrived).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDraftUpdateSignal {
    pub session_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
}

pub type DraftSyncEmit = Arc<dyn Fn(NativeDraftUpdateSignal) + Send + Sync>;

#[derive(Debug, Clone)]
struct PendingWrite {
    slot: SynaraDraftSlot,
    generation: u64,
    first_queued: Instant,
}

struct Inner {
    client: Client,
    session_generation: u64,
    enabled: AtomicBool,
    emit: DraftSyncEmit,
    /// Payloads this device wrote, until sync carries them back. Merged per
    /// slot with last-writer-wins, so a stale overlay can never win.
    overlay: Mutex<HashMap<OwnedRoomId, SynaraDraftsPayload>>,
    /// Highest `updated_at` seen per (room, slot), for the Lamport stamp.
    seen: Mutex<HashMap<(OwnedRoomId, String), u64>>,
    pending: Mutex<HashMap<(OwnedRoomId, String), PendingWrite>>,
    next_generation: AtomicU64,
    /// One sealed read-modify-write at a time.
    write_lock: AsyncMutex<()>,
    peers: PeerShareState,
    last_peer_request: Mutex<Option<Instant>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub struct DraftSyncOwner {
    inner: Arc<Inner>,
    _room_account_data: EventHandlerDropGuard,
    _to_device: EventHandlerDropGuard,
    epoch_task: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for DraftSyncOwner {
    fn drop(&mut self) {
        if let Some(task) = self.epoch_task.take() {
            task.abort();
        }
    }
}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    RoomId::parse(room_id).map_err(|_| "drafts-invalid-room")
}

fn bounded_event_id(value: Option<String>) -> Result<Option<String>, &'static str> {
    match value {
        Some(id) if !id.starts_with('$') || id.len() > MAX_DRAFT_EVENT_ID_BYTES => {
            Err("drafts-invalid-event-id")
        }
        other => Ok(other),
    }
}

#[derive(Deserialize)]
struct RawAccountDataEvent {
    content: Box<RawJsonValue>,
}

impl DraftSyncOwner {
    pub fn start(
        client: &Client,
        emit: DraftSyncEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        client.user_id().ok_or("drafts-no-user")?;
        let inner = Arc::new(Inner {
            client: client.clone(),
            session_generation,
            enabled: AtomicBool::new(true),
            emit,
            overlay: Mutex::new(HashMap::new()),
            seen: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            next_generation: AtomicU64::new(0),
            write_lock: AsyncMutex::new(()),
            peers: PeerShareState::default(),
            last_peer_request: Mutex::new(None),
        });

        let account_inner = Arc::clone(&inner);
        let room_handle = client.add_event_handler(
            move |event: AnyRoomAccountDataEvent, room: Room, raw: RawEvent| {
                let inner = Arc::clone(&account_inner);
                async move {
                    if event.event_type().to_string() != DRAFTS_EVENT_TYPE {
                        return;
                    }
                    inner.observe_synced(room.room_id(), &raw).await;
                }
            },
        );

        let device_inner = Arc::clone(&inner);
        let to_device_handle = client.add_event_handler(
            move |raw: Raw<AnyToDeviceEvent>, info: Option<EncryptionInfo>| {
                let inner = Arc::clone(&device_inner);
                async move {
                    if !inner.enabled.load(Ordering::Relaxed) {
                        return;
                    }
                    // A received key bumps the key epoch; its watcher emits
                    // the wakeup.
                    key::handle_to_device(&inner.client, &inner.peers, &raw, info.as_ref()).await;
                }
            },
        );

        let epoch_inner = Arc::clone(&inner);
        let mut epoch = key::subscribe_key_epoch();
        // Owners start inside the shell's runtime; without one (unit tests)
        // there is simply no key-change wakeup.
        let epoch_task = tokio::runtime::Handle::try_current().ok().map(|runtime| {
            runtime.spawn(async move {
                while epoch.changed().await.is_ok() {
                    (epoch_inner.emit)(NativeDraftUpdateSignal {
                        session_generation: epoch_inner.session_generation,
                        room_id: None,
                    });
                }
            })
        });

        Ok(Self {
            inner,
            _room_account_data: client.event_handler_drop_guard(room_handle),
            _to_device: client.event_handler_drop_guard(to_device_handle),
            epoch_task,
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.inner.session_generation
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.inner.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            lock(&self.inner.pending).clear();
        }
    }

    pub async fn status(&self) -> NativeDraftSyncStatus {
        let inner = &self.inner;
        let enabled = inner.enabled.load(Ordering::Relaxed);
        let key_state = if !enabled {
            NativeDraftKeyState::Disabled
        } else if key::load_cached_key(&inner.client).await.is_some() {
            NativeDraftKeyState::Ready
        } else if inner.request_key_from_peers().await {
            NativeDraftKeyState::Requesting
        } else {
            NativeDraftKeyState::NeedsRecovery
        };
        NativeDraftSyncStatus {
            session_generation: inner.session_generation,
            enabled,
            key_state,
        }
    }

    /// Queue a composer's draft. Empty content syncs as a cleared draft.
    /// `flush` writes now (room switch, blur, background).
    #[allow(clippy::too_many_arguments)]
    pub async fn save(
        &self,
        room_id: &str,
        slot: &str,
        markdown: Option<String>,
        desktop_slate: Option<serde_json::Value>,
        reply_to: Option<String>,
        edit_of: Option<String>,
        flush: bool,
    ) -> Result<u64, &'static str> {
        let inner = &self.inner;
        if !inner.enabled.load(Ordering::Relaxed) {
            return Err("drafts-sync-disabled");
        }
        let room = parse_room_id(room_id)?;
        validate_slot_key(slot)?;
        let reply_to = bounded_event_id(reply_to)?;
        let edit_of = bounded_event_id(edit_of)?;
        if markdown
            .as_ref()
            .is_some_and(|m| m.len() > MAX_MARKDOWN_BYTES)
        {
            return Err("drafts-markdown-too-large");
        }
        let desktop_slate = match desktop_slate {
            Some(value)
                if serde_json::to_vec(&value).map_or(usize::MAX, |b| b.len()) > MAX_SLATE_BYTES =>
            {
                None
            }
            other => other,
        };
        let device_id = inner.own_device_id()?;
        let updated_at = inner.stamp(&room, slot);
        let empty = markdown.as_deref().is_none_or(|m| m.trim().is_empty())
            && reply_to.is_none()
            && edit_of.is_none();
        let slot_value = if empty {
            SynaraDraftSlot::tombstone(updated_at, &device_id)
        } else {
            SynaraDraftSlot {
                updated_at,
                device_id,
                cleared: false,
                markdown,
                desktop_slate,
                reply_to,
                edit_of,
                truncated: false,
            }
        };
        inner
            .queue(room, slot.to_owned(), slot_value, flush)
            .await?;
        Ok(updated_at)
    }

    /// The draft was sent: clear it on every device now.
    pub async fn clear(&self, room_id: &str, slot: &str) -> Result<u64, &'static str> {
        let inner = &self.inner;
        if !inner.enabled.load(Ordering::Relaxed) {
            return Err("drafts-sync-disabled");
        }
        let room = parse_room_id(room_id)?;
        validate_slot_key(slot)?;
        let updated_at = inner.stamp(&room, slot);
        let tombstone = SynaraDraftSlot::tombstone(updated_at, &inner.own_device_id()?);
        inner.queue(room, slot.to_owned(), tombstone, true).await?;
        Ok(updated_at)
    }

    /// Resolve the synced draft for one composer against its local state.
    pub async fn resolve(
        &self,
        room_id: &str,
        slot: &str,
        local: NativeLocalDraftState,
        fetch_fresh: bool,
    ) -> Result<NativeDraftResolution, &'static str> {
        let inner = &self.inner;
        let keep = NativeDraftResolution {
            session_generation: inner.session_generation,
            decision: NativeDraftDecision::Keep,
            draft: None,
        };
        if !inner.enabled.load(Ordering::Relaxed) {
            return Ok(keep);
        }
        let room = parse_room_id(room_id)?;
        validate_slot_key(slot)?;
        let Some(key) = key::load_cached_key(&inner.client).await else {
            return Ok(keep);
        };
        let payload = inner.room_payload(&room, &key, fetch_fresh).await?;
        let Some(remote) = payload.slots.get(slot).cloned() else {
            return Ok(keep);
        };
        let own_device = inner.own_device_id()?;
        let decision = decide_remote(
            &LocalDraftState {
                updated_at: local.updated_at,
                dirty: local.dirty,
                markdown: local.markdown,
            },
            &remote,
            &own_device,
        );
        Ok(NativeDraftResolution {
            session_generation: inner.session_generation,
            decision: decision.into(),
            draft: (decision != DraftDecision::Keep)
                .then(|| NativeRemoteDraft::from_slot(room_id, slot, remote)),
        })
    }

    /// Clear every synced draft of this account (the setting's delete action).
    pub async fn delete_all(&self) -> Result<u32, &'static str> {
        let inner = &self.inner;
        let Some(key) = key::load_cached_key(&inner.client).await else {
            return Err("drafts-key-missing");
        };
        let device_id = inner.own_device_id()?;
        let mut cleared = 0u32;
        for room in inner.client.joined_rooms() {
            let room_id = room.room_id().to_owned();
            let Ok(payload) = inner.room_payload(&room_id, &key, false).await else {
                continue;
            };
            let live: Vec<String> = payload
                .slots
                .iter()
                .filter(|(_, slot)| !slot.cleared)
                .map(|(slot_key, _)| slot_key.clone())
                .collect();
            for slot_key in live {
                let tombstone =
                    SynaraDraftSlot::tombstone(inner.stamp(&room_id, &slot_key), &device_id);
                if inner
                    .queue(room_id.clone(), slot_key, tombstone, true)
                    .await
                    .is_ok()
                {
                    cleared = cleared.saturating_add(1);
                }
            }
        }
        Ok(cleared)
    }

    /// Write everything queued now (app background / quit).
    pub async fn flush_all(&self) {
        let keys: Vec<(OwnedRoomId, String)> = lock(&self.inner.pending).keys().cloned().collect();
        for (room, slot) in keys {
            let _ = self.inner.write_pending(&room, &slot, None).await;
        }
    }
}

impl Inner {
    fn own_device_id(&self) -> Result<String, &'static str> {
        self.client
            .device_id()
            .map(|id| id.to_string())
            .ok_or("drafts-no-device")
    }

    fn observe(&self, room: &RoomId, payload: &SynaraDraftsPayload) {
        let mut seen = lock(&self.seen);
        for (slot_key, slot) in &payload.slots {
            let entry = seen.entry((room.to_owned(), slot_key.clone())).or_insert(0);
            *entry = (*entry).max(slot.updated_at);
        }
    }

    fn stamp(&self, room: &RoomId, slot: &str) -> u64 {
        let mut seen = lock(&self.seen);
        let entry = seen.entry((room.to_owned(), slot.to_owned())).or_insert(0);
        let stamp = next_updated_at(now_ms(), (*entry > 0).then_some(*entry));
        *entry = stamp;
        stamp
    }

    async fn request_key_from_peers(&self) -> bool {
        {
            let mut last = lock(&self.last_peer_request);
            if last.is_some_and(|at| at.elapsed() < PEER_REQUEST_INTERVAL) {
                return true;
            }
            *last = Some(Instant::now());
        }
        let asked = key::request_from_peers(&self.client, &self.peers)
            .await
            .unwrap_or(0);
        if asked == 0 {
            // Nobody to ask: allow an immediate retry once a device appears.
            *lock(&self.last_peer_request) = None;
        }
        asked > 0
    }

    fn decode(&self, room: &RoomId, key: &DraftsKey, content: &str) -> Option<SynaraDraftsPayload> {
        let user_id = self.client.user_id()?;
        let envelope = parse_envelope(content).ok()?;
        // Another key (pre-reset, or a device that has not converged), a newer
        // format or tampering all read as absent; our next write replaces it.
        open_envelope(key, user_id.as_str(), room.as_str(), &envelope).ok()
    }

    async fn cached_payload(&self, room: &RoomId, key: &DraftsKey) -> SynaraDraftsPayload {
        let Some(sdk_room) = self.client.get_room(room) else {
            return SynaraDraftsPayload::default();
        };
        let Ok(Some(raw)) = sdk_room.account_data(drafts_type()).await else {
            return SynaraDraftsPayload::default();
        };
        let Ok(event) = raw.deserialize_as_unchecked::<RawAccountDataEvent>() else {
            return SynaraDraftsPayload::default();
        };
        self.decode(room, key, event.content.get())
            .unwrap_or_default()
    }

    async fn fresh_payload(
        &self,
        room: &RoomId,
        key: &DraftsKey,
    ) -> Result<SynaraDraftsPayload, &'static str> {
        let user_id = self.client.user_id().ok_or("drafts-no-user")?;
        let request = get_room_account_data::v3::Request::new(
            user_id.to_owned(),
            room.to_owned(),
            drafts_type(),
        );
        match self.client.send(request).await {
            Ok(response) => Ok(self
                .decode(room, key, response.account_data.json().get())
                .unwrap_or_default()),
            Err(error) if error.client_api_error_kind() == Some(&ErrorKind::NotFound) => {
                Ok(SynaraDraftsPayload::default())
            }
            Err(_) => Err("drafts-fetch-failed"),
        }
    }

    /// Store (or homeserver) ∪ this device's unsynced writes.
    async fn room_payload(
        &self,
        room: &RoomId,
        key: &DraftsKey,
        fetch_fresh: bool,
    ) -> Result<SynaraDraftsPayload, &'static str> {
        let base = if fetch_fresh {
            self.fresh_payload(room, key).await?
        } else {
            self.cached_payload(room, key).await
        };
        let merged = match lock(&self.overlay).get(room) {
            Some(overlay) => merge_payloads(&base, overlay),
            None => base,
        };
        self.observe(room, &merged);
        Ok(merged)
    }

    async fn observe_synced(&self, room: &RoomId, raw: &RawJsonValue) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let Some(key) = key::load_cached_key(&self.client).await else {
            return;
        };
        let Ok(event) = serde_json::from_str::<RawAccountDataEvent>(raw.get()) else {
            return;
        };
        let Some(payload) = self.decode(room, &key, event.content.get()) else {
            return;
        };
        self.observe(room, &payload);
        let own = self.client.device_id().map(|id| id.as_str().to_owned());
        {
            // Sync now carries what we wrote: retire overlay slots it covers.
            let mut overlay = lock(&self.overlay);
            if let Some(entry) = overlay.get_mut(room) {
                entry.slots.retain(|slot_key, ours| {
                    payload
                        .slots
                        .get(slot_key)
                        .is_none_or(|synced| ours.is_newer_than(synced))
                });
                if entry.slots.is_empty() {
                    overlay.remove(room);
                }
            }
        }
        let from_peer = payload
            .slots
            .values()
            .any(|slot| Some(slot.device_id.as_str()) != own.as_deref());
        if from_peer {
            (self.emit)(NativeDraftUpdateSignal {
                session_generation: self.session_generation,
                room_id: Some(room.to_string()),
            });
        }
    }

    async fn queue(
        self: &Arc<Self>,
        room: OwnedRoomId,
        slot: String,
        value: SynaraDraftSlot,
        flush: bool,
    ) -> Result<(), &'static str> {
        if key::load_cached_key(&self.client).await.is_none() {
            return Err("drafts-key-missing");
        }
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let first_queued = {
            let mut pending = lock(&self.pending);
            let first = pending
                .get(&(room.clone(), slot.clone()))
                .map_or_else(Instant::now, |existing| existing.first_queued);
            pending.insert(
                (room.clone(), slot.clone()),
                PendingWrite {
                    slot: value,
                    generation,
                    first_queued: first,
                },
            );
            first
        };
        if flush {
            return self.write_pending(&room, &slot, None).await;
        }
        let inner = Arc::clone(self);
        tokio::spawn(async move {
            let wait = SAVE_DEBOUNCE.min(SAVE_MAX_DELAY.saturating_sub(first_queued.elapsed()));
            tokio::time::sleep(wait).await;
            let _ = inner.write_pending(&room, &slot, Some(generation)).await;
        });
        Ok(())
    }

    /// Write the queued slot. With `only_generation` (a debounce timer), skip
    /// when a newer save superseded it, unless the slot has waited
    /// [`SAVE_MAX_DELAY`]: then the first timer to wake writes the latest.
    async fn write_pending(
        &self,
        room: &RoomId,
        slot: &str,
        only_generation: Option<u64>,
    ) -> Result<(), &'static str> {
        let _guard = self.write_lock.lock().await;
        let pending = {
            let pending = lock(&self.pending);
            match pending.get(&(room.to_owned(), slot.to_owned())) {
                None => return Ok(()),
                Some(entry)
                    if only_generation.is_some_and(|generation| generation != entry.generation)
                        && entry.first_queued.elapsed() < SAVE_MAX_DELAY =>
                {
                    return Ok(());
                }
                Some(entry) => entry.clone(),
            }
        };
        let result = self.write_slot(room, slot, pending.slot.clone()).await;
        let mut queued = lock(&self.pending);
        let key = (room.to_owned(), slot.to_owned());
        match queued.get_mut(&key) {
            Some(entry) if entry.generation == pending.generation => {
                queued.remove(&key);
            }
            // Typed on during the write: its own wait starts now, so a long
            // typing burst writes every few seconds, not on every keystroke.
            Some(entry) => entry.first_queued = Instant::now(),
            None => {}
        }
        result
    }

    async fn write_slot(
        &self,
        room: &RoomId,
        slot_key: &str,
        slot: SynaraDraftSlot,
    ) -> Result<(), &'static str> {
        let key = key::load_cached_key(&self.client)
            .await
            .ok_or("drafts-key-missing")?;
        let user_id = self.client.user_id().ok_or("drafts-no-user")?.to_owned();
        let sdk_room = self.client.get_room(room).ok_or("drafts-room-unknown")?;
        // Fresh read: a peer may have written another thread's slot that our
        // sync has not delivered yet.
        let mut payload = self.room_payload(room, &key, true).await?;
        if payload
            .slots
            .get(slot_key)
            .is_some_and(|existing| existing.is_newer_than(&slot))
        {
            // A peer already wrote something newer; the shell resolves it.
            return Ok(());
        }
        payload.v = DRAFTS_PAYLOAD_VERSION;
        payload.slots.insert(slot_key.to_owned(), slot);
        let now = now_ms();
        prune_tombstones(&mut payload, now);
        // Truncation is recorded on the slot for receiving devices.
        cap_payload(&mut payload, slot_key, now);
        let envelope = seal_payload(&key, user_id.as_str(), room.as_str(), &payload)?;
        let raw = to_raw_value(&envelope).map_err(|_| "drafts-serialize-failed")?;
        sdk_room
            .set_account_data_raw(
                drafts_type(),
                Raw::<AnyRoomAccountDataEventContent>::from_json(raw),
            )
            .await
            .map_err(|_| "drafts-write-failed")?;
        let mut overlay = lock(&self.overlay);
        let entry = overlay.entry(room.to_owned()).or_default();
        *entry = merge_payloads(entry, &payload);
        Ok(())
    }
}

/// Secret storage was just enrolled, unlocked or re-keyed with `secret`.
pub(crate) async fn on_secret_store_open(client: &Client, secret: &str) {
    key::on_secret_store_open(client, secret).await;
}

/// A cryptographic identity reset completed.
pub(crate) async fn on_identity_reset(client: &Client) {
    key::on_identity_reset(client).await;
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
