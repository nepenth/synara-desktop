//! Live im.ponies image-pack Client snapshot/set and subscribe owner.
//!
//! Shells supply the emit sink (desktop Tauri event / later iOS UniFFI).

use std::{
    future::Future,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use matrix_sdk::event_handler::{EventHandlerDropGuard, RawEvent};
use matrix_sdk::ruma::events::{AnyGlobalAccountDataEvent, AnySyncStateEvent};
use matrix_sdk::{
    deserialized_responses::RawAnySyncOrStrippedState,
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType, StateEventType},
        serde::Raw,
        OwnedRoomId, RoomId,
    },
    Client,
};
use serde::Serialize;
use serde_json::value::to_raw_value;
use serde_json::Value as JsonValue;
use tokio::sync::Mutex as AsyncMutex;

use super::{
    is_image_pack_account_data_type, is_image_pack_room_state_type, pack_from_account_data,
    set_global_image_packs_content_guard, set_room_image_pack_content_guard,
    set_user_image_pack_content_guard, EmoteRoomsContent, NativeGlobalImagePacksSnapshot,
    NativeImagePack, NativeLaterSnapshot, NativeMDirectMutationResult, NativeMDirectSnapshot,
    NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot, NativeUserImagePackSnapshot,
    RoomNoteMoveDirection, SynaraLaterItem, SynaraRoomNoteItem, SynaraRoomNotesContent,
    EMOTE_ROOMS_EVENT_TYPE, ROOM_EMOTES_EVENT_TYPE, ROOM_NOTES_EVENT_TYPE, USER_EMOTES_EVENT_TYPE,
};

const ROOM_NOTES_PENDING_PROJECTION_TTL: Duration = Duration::from_secs(30);

#[derive(Default)]
struct RoomNotesProjectionState {
    mutation_in_flight: bool,
    pending: Option<PendingRoomNotesProjection>,
    synchronized: Option<SynchronizedRoomNotesProjection>,
}

struct PendingRoomNotesProjection {
    content: SynaraRoomNotesContent,
    stored_at: Instant,
}

struct SynchronizedRoomNotesProjection {
    content: Result<SynaraRoomNotesContent, &'static str>,
    observed_at: Instant,
}

impl RoomNotesProjectionState {
    fn begin_mutation(&mut self) {
        self.mutation_in_flight = true;
    }

    fn finish_mutation(&mut self, content: Option<SynaraRoomNotesContent>, now: Instant) {
        self.mutation_in_flight = false;
        if let Some(content) = content {
            self.pending = Some(PendingRoomNotesProjection {
                content,
                stored_at: now,
            });
        }
    }

    fn observe_synchronized_event(
        &mut self,
        content: Result<SynaraRoomNotesContent, &'static str>,
        now: Instant,
    ) {
        // A sync delivered while the serialized RMW is still running may
        // describe server state from before the successful PUT. The local
        // write completes later and therefore remains the visible projection.
        if self.mutation_in_flight {
            return;
        }

        // While a successful local write is pending acknowledgement, a
        // non-matching account-data event is ambiguous: it may be a legitimate
        // later writer, or a delayed pre-PUT /sync response. Do not cache that
        // event beyond the local overlay. Once the bounded pending window
        // expires, `project` returns the SDK's current synchronized snapshot,
        // which is the only source that can disambiguate the winner.
        if let Some(pending) = self.pending.as_ref() {
            let acknowledges_pending = content.as_ref().is_ok_and(|next| next == &pending.content);
            if !acknowledges_pending {
                return;
            }
        }

        self.synchronized = Some(SynchronizedRoomNotesProjection {
            content,
            observed_at: now,
        });
        if self.pending.is_some() {
            self.pending = None;
        }
    }

    fn project(
        &mut self,
        synchronized: Result<SynaraRoomNotesContent, &'static str>,
        now: Instant,
    ) -> Result<SynaraRoomNotesContent, &'static str> {
        if let Some(pending) = self.pending.as_ref() {
            if now.saturating_duration_since(pending.stored_at) < ROOM_NOTES_PENDING_PROJECTION_TTL
            {
                return Ok(pending.content.clone());
            }
            self.pending = None;
        }

        if let Some(observed) = self.synchronized.as_ref() {
            if now.saturating_duration_since(observed.observed_at)
                < ROOM_NOTES_PENDING_PROJECTION_TTL
            {
                return observed.content.clone();
            }
            self.synchronized = None;
        }
        synchronized
    }
}

fn lock_room_notes_projection(
    state: &Mutex<RoomNotesProjectionState>,
) -> MutexGuard<'_, RoomNotesProjectionState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct RoomNotesMutationProjectionGuard {
    state: Arc<Mutex<RoomNotesProjectionState>>,
    finished: bool,
}

impl RoomNotesMutationProjectionGuard {
    fn begin(state: Arc<Mutex<RoomNotesProjectionState>>) -> Self {
        lock_room_notes_projection(&state).begin_mutation();
        Self {
            state,
            finished: false,
        }
    }

    fn finish(mut self, content: Option<SynaraRoomNotesContent>) {
        lock_room_notes_projection(&self.state).finish_mutation(content, Instant::now());
        self.finished = true;
    }
}

impl Drop for RoomNotesMutationProjectionGuard {
    fn drop(&mut self) {
        if !self.finished {
            lock_room_notes_projection(&self.state).mutation_in_flight = false;
        }
    }
}

/// Shell-supplied sink for image-pack wakeups.
pub type ImagePackUpdateEmit = Arc<dyn Fn(NativeImagePackUpdateSignal) + Send + Sync>;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePackUpdateSignal {
    pub session_generation: u64,
}

fn user_emotes_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(USER_EMOTES_EVENT_TYPE)
}

fn emote_rooms_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(EMOTE_ROOMS_EVENT_TYPE)
}

fn room_emotes_type() -> StateEventType {
    StateEventType::from(ROOM_EMOTES_EVENT_TYPE)
}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    RoomId::parse(room_id).map_err(|_| "v-send.r-pack-read-invalid-room")
}

async fn load_account_data_value(
    client: &Client,
    event_type: GlobalAccountDataEventType,
) -> Result<Option<JsonValue>, &'static str> {
    let raw = client
        .account()
        .account_data_raw(event_type)
        .await
        .map_err(|_| "v-send.r-pack-read-fetch-failed")?;
    match raw {
        Some(raw) => raw
            .deserialize_as_unchecked::<JsonValue>()
            .map(Some)
            .map_err(|_| "v-send.r-pack-read-deserialize-failed"),
        None => Ok(None),
    }
}

fn extract_sync_state_pack(
    room_id: &str,
    raw: &RawAnySyncOrStrippedState,
) -> Option<NativeImagePack> {
    let RawAnySyncOrStrippedState::Sync(raw_ev) = raw else {
        return None;
    };
    let value: JsonValue = raw_ev.deserialize_as_unchecked().ok()?;
    let state_key = value.get("state_key")?.as_str()?.to_owned();
    let event_id = value
        .get("event_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{room_id}:{state_key}"));
    let content = value.get("content")?.clone();
    if !content.is_object() {
        return None;
    }
    Some(NativeImagePack {
        id: event_id,
        room_id: Some(room_id.to_owned()),
        state_key: Some(state_key),
        content,
    })
}

async fn load_room_packs(
    client: &Client,
    room_id: &RoomId,
) -> Result<Vec<NativeImagePack>, &'static str> {
    let room = client
        .get_room(room_id)
        .ok_or("v-send.r-pack-read-room-missing")?;
    let raw_events = room
        .get_state_events(room_emotes_type())
        .await
        .map_err(|_| "v-send.r-pack-read-state-fetch-failed")?;
    let mut packs = Vec::new();
    for raw in raw_events {
        if let Some(pack) = extract_sync_state_pack(room_id.as_str(), &raw) {
            packs.push(pack);
        }
    }
    packs.sort_by(|a, b| a.state_key.cmp(&b.state_key));
    Ok(packs)
}

pub async fn snapshot_user_image_pack(
    client: &Client,
    session_generation: u64,
) -> Result<NativeUserImagePackSnapshot, &'static str> {
    let user_id = client
        .user_id()
        .ok_or("v-send.r-pack-read-no-user")?
        .to_string();
    let content = load_account_data_value(client, user_emotes_type()).await?;
    let pack = content.map(|c| pack_from_account_data(user_id, c));
    Ok(NativeUserImagePackSnapshot {
        session_generation,
        pack,
    })
}

pub async fn snapshot_room_image_packs(
    client: &Client,
    session_generation: u64,
    room_id: &str,
) -> Result<NativeRoomImagePacksSnapshot, &'static str> {
    let room_id = parse_room_id(room_id)?;
    let packs = load_room_packs(client, &room_id).await?;
    Ok(NativeRoomImagePacksSnapshot {
        session_generation,
        room_id: room_id.to_string(),
        packs,
    })
}

pub async fn snapshot_global_image_packs(
    client: &Client,
    session_generation: u64,
) -> Result<NativeGlobalImagePacksSnapshot, &'static str> {
    let raw = load_account_data_value(client, emote_rooms_type()).await?;
    let emote_rooms: EmoteRoomsContent = match raw {
        Some(value) => {
            serde_json::from_value(value).map_err(|_| "v-send.r-pack-read-deserialize-failed")?
        }
        None => EmoteRoomsContent::default(),
    };

    let mut packs = Vec::new();
    for (room_id_str, enabled_keys) in emote_rooms.rooms {
        let Ok(room_id) = RoomId::parse(&room_id_str) else {
            continue;
        };
        if client.get_room(&room_id).is_none() {
            continue;
        }
        let room_packs = match load_room_packs(client, &room_id).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        for pack in room_packs {
            let Some(state_key) = pack.state_key.as_deref() else {
                continue;
            };
            if enabled_keys.contains_key(state_key) {
                packs.push(pack);
            }
        }
    }
    packs.sort_by(|a, b| {
        (
            a.room_id.as_deref().unwrap_or(""),
            a.state_key.as_deref().unwrap_or(""),
        )
            .cmp(&(
                b.room_id.as_deref().unwrap_or(""),
                b.state_key.as_deref().unwrap_or(""),
            ))
    });
    Ok(NativeGlobalImagePacksSnapshot {
        session_generation,
        packs,
    })
}

pub async fn set_user_image_pack(client: &Client, content: JsonValue) -> Result<(), &'static str> {
    set_user_image_pack_content_guard(&content)?;
    let raw_value = to_raw_value(&content).map_err(|_| "v-send.r-pack-write-serialize-failed")?;
    let raw = Raw::<AnyGlobalAccountDataEventContent>::from_json(raw_value);
    client
        .account()
        .set_account_data_raw(user_emotes_type(), raw)
        .await
        .map_err(|_| "v-send.r-pack-write-set-failed")?;
    Ok(())
}

pub async fn set_global_image_packs(
    client: &Client,
    content: JsonValue,
) -> Result<(), &'static str> {
    set_global_image_packs_content_guard(&content)?;
    let raw_value = to_raw_value(&content).map_err(|_| "v-send.r-pack-write-serialize-failed")?;
    let raw = Raw::<AnyGlobalAccountDataEventContent>::from_json(raw_value);
    client
        .account()
        .set_account_data_raw(emote_rooms_type(), raw)
        .await
        .map_err(|_| "v-send.r-pack-write-set-failed")?;
    Ok(())
}

pub async fn set_room_image_pack(
    client: &Client,
    room_id: &str,
    state_key: &str,
    content: JsonValue,
) -> Result<(), &'static str> {
    set_room_image_pack_content_guard(&content)?;
    let room_id = parse_room_id(room_id)?;
    let room = client
        .get_room(&room_id)
        .ok_or("v-send.r-pack-write-room-missing")?;
    room.send_state_event_raw(ROOM_EMOTES_EVENT_TYPE, state_key, content)
        .await
        .map_err(|_| "v-send.r-pack-write-set-failed")?;
    Ok(())
}

/// V-SEND.R-PACK-READ subscribe: live push of pack account-data/state changes.
pub struct NativeImagePackOwner {
    client: Client,
    session_generation: u64,
    pending_threepid: Mutex<Option<crate::app::user_profile::PendingThreepid>>,
    // Matrix global account data has no conditional-write primitive. Keep the
    // v1 notes RMW route single-writer within this live Core owner.
    room_notes_mutation: AsyncMutex<()>,
    // A successful PUT is not inserted into matrix-sdk's synchronized store.
    // Keep its result visible until a subsequent notes /sync event supersedes
    // it (or a short fail-safe expiry prevents an unbounded local overlay).
    room_notes_projection: Arc<Mutex<RoomNotesProjectionState>>,
    _account_data: EventHandlerDropGuard,
    _state: EventHandlerDropGuard,
}

impl NativeImagePackOwner {
    pub fn start(
        client: &Client,
        emit: ImagePackUpdateEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        let _ = client
            .user_id()
            .ok_or("v-send.r-pack-read-subscribe-no-user")?;

        let room_notes_projection = Arc::new(Mutex::new(RoomNotesProjectionState::default()));
        let account_projection = Arc::clone(&room_notes_projection);
        let emit_account = emit.clone();
        let account_handle =
            client.add_event_handler(move |event: AnyGlobalAccountDataEvent, raw: RawEvent| {
                let emit = emit_account.clone();
                let projection = Arc::clone(&account_projection);
                async move {
                    let event_type = event.event_type().to_string();
                    if event_type == ROOM_NOTES_EVENT_TYPE {
                        let content = super::room_notes_live::parse_room_notes_sync_event(&raw);
                        lock_room_notes_projection(&projection)
                            .observe_synchronized_event(content, Instant::now());
                    }
                    if is_image_pack_account_data_type(&event_type) {
                        emit(NativeImagePackUpdateSignal { session_generation });
                    }
                }
            });

        let emit_state = emit;
        let state_handle = client.add_event_handler(move |event: AnySyncStateEvent| {
            let emit = emit_state.clone();
            async move {
                let event_type = event.event_type().to_string();
                if is_image_pack_room_state_type(&event_type) {
                    emit(NativeImagePackUpdateSignal { session_generation });
                }
            }
        });

        Ok(Self {
            client: client.clone(),
            session_generation,
            pending_threepid: Mutex::new(None),
            room_notes_mutation: AsyncMutex::new(()),
            room_notes_projection,
            _account_data: client.event_handler_drop_guard(account_handle),
            _state: client.event_handler_drop_guard(state_handle),
        })
    }

    pub async fn snapshot_global(&self) -> Result<NativeGlobalImagePacksSnapshot, &'static str> {
        snapshot_global_image_packs(&self.client, self.session_generation).await
    }

    pub async fn snapshot_user(&self) -> Result<NativeUserImagePackSnapshot, &'static str> {
        snapshot_user_image_pack(&self.client, self.session_generation).await
    }

    pub async fn snapshot_room(
        &self,
        room_id: &str,
    ) -> Result<NativeRoomImagePacksSnapshot, &'static str> {
        snapshot_room_image_packs(&self.client, self.session_generation, room_id).await
    }

    pub async fn set_user(&self, content: JsonValue) -> Result<(), &'static str> {
        set_user_image_pack(&self.client, content).await
    }

    pub async fn set_global(&self, content: JsonValue) -> Result<(), &'static str> {
        set_global_image_packs(&self.client, content).await
    }

    pub async fn mdirect_snapshot(&self) -> Result<NativeMDirectSnapshot, &'static str> {
        super::snapshot_mdirect(&self.client, self.session_generation).await
    }

    pub async fn mdirect_add(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> Result<NativeMDirectMutationResult, &'static str> {
        super::add_room_to_mdirect(&self.client, room_id, user_id).await
    }

    pub async fn mdirect_remove(
        &self,
        room_id: &str,
    ) -> Result<NativeMDirectMutationResult, &'static str> {
        super::remove_room_from_mdirect(&self.client, room_id).await
    }

    pub async fn later_snapshot(&self) -> Result<NativeLaterSnapshot, &'static str> {
        super::snapshot_later(&self.client, self.session_generation).await
    }

    pub async fn later_upsert(
        &self,
        item: SynaraLaterItem,
    ) -> Result<NativeLaterSnapshot, &'static str> {
        super::upsert_later_item(&self.client, self.session_generation, item).await
    }

    pub async fn later_complete(
        &self,
        item_id: String,
        completed_at: Option<f64>,
    ) -> Result<NativeLaterSnapshot, &'static str> {
        super::complete_later_item_live(
            &self.client,
            self.session_generation,
            item_id,
            super::later_timestamp_or_now(completed_at),
        )
        .await
    }

    pub async fn later_snooze(
        &self,
        item_id: String,
        due_ts: f64,
    ) -> Result<NativeLaterSnapshot, &'static str> {
        super::snooze_later_item_live(&self.client, self.session_generation, item_id, due_ts).await
    }

    pub async fn later_clear_completed(&self) -> Result<NativeLaterSnapshot, &'static str> {
        super::clear_completed_later_live(&self.client, self.session_generation).await
    }

    pub async fn later_mark_reminded(
        &self,
        item_id: String,
        reminded_at: Option<f64>,
    ) -> Result<NativeLaterSnapshot, &'static str> {
        super::mark_later_reminded_live(
            &self.client,
            self.session_generation,
            item_id,
            super::later_timestamp_or_now(reminded_at),
        )
        .await
    }

    pub async fn room_notes_snapshot(&self) -> Result<NativeRoomNotesSnapshot, &'static str> {
        let synchronized = super::snapshot_room_notes(&self.client, self.session_generation)
            .await
            .map(|snapshot| snapshot.content);
        let content = lock_room_notes_projection(&self.room_notes_projection)
            .project(synchronized, Instant::now())?;
        Ok(NativeRoomNotesSnapshot {
            session_generation: self.session_generation,
            content,
        })
    }

    async fn project_room_notes_mutation<F>(
        &self,
        mutation: F,
    ) -> Result<NativeRoomNotesSnapshot, &'static str>
    where
        F: Future<Output = Result<NativeRoomNotesSnapshot, &'static str>>,
    {
        let _mutation_guard = self.room_notes_mutation.lock().await;
        let projection_guard =
            RoomNotesMutationProjectionGuard::begin(Arc::clone(&self.room_notes_projection));
        let result = mutation.await;
        projection_guard.finish(
            result
                .as_ref()
                .ok()
                .map(|snapshot| snapshot.content.clone()),
        );
        result
    }

    pub async fn room_notes_upsert(
        &self,
        item: SynaraRoomNoteItem,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        self.project_room_notes_mutation(super::upsert_room_note_item(
            &self.client,
            self.session_generation,
            item,
        ))
        .await
    }

    pub async fn room_notes_delete(
        &self,
        room_id: String,
        item_id: String,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        self.project_room_notes_mutation(super::delete_room_note_item_live(
            &self.client,
            self.session_generation,
            room_id,
            item_id,
        ))
        .await
    }

    pub async fn room_notes_complete_todo(
        &self,
        room_id: String,
        item_id: String,
        completed: bool,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        self.project_room_notes_mutation(super::complete_room_todo_item_live(
            &self.client,
            self.session_generation,
            room_id,
            item_id,
            completed,
            super::room_notes_now_ms(),
        ))
        .await
    }

    pub async fn room_notes_move_todo(
        &self,
        room_id: String,
        item_id: String,
        direction: RoomNoteMoveDirection,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        self.project_room_notes_mutation(super::move_room_todo_item_live(
            &self.client,
            self.session_generation,
            room_id,
            item_id,
            direction,
            super::room_notes_now_ms(),
        ))
        .await
    }

    pub async fn set_room(
        &self,
        room_id: &str,
        state_key: &str,
        content: JsonValue,
    ) -> Result<(), &'static str> {
        set_room_image_pack(&self.client, room_id, state_key, content).await
    }

    pub async fn set_own_display_name(
        &self,
        display_name: &str,
    ) -> Result<crate::app::user_profile::MatrixProfileWriteResult, &'static str> {
        crate::app::user_profile::set_own_display_name(&self.client, display_name).await
    }

    pub async fn set_own_avatar(
        &self,
        mxc: &str,
    ) -> Result<crate::app::user_profile::MatrixProfileWriteResult, &'static str> {
        crate::app::user_profile::set_own_avatar(&self.client, mxc).await
    }

    pub async fn get_own_profile(
        &self,
    ) -> Result<crate::app::user_profile::MatrixOwnProfile, &'static str> {
        crate::app::user_profile::get_own_profile(&self.client).await
    }

    pub async fn snapshot_ignored_users(
        &self,
    ) -> Result<crate::app::user_profile::MatrixIgnoredUsersSnapshot, &'static str> {
        crate::app::user_profile::snapshot_ignored_users(&self.client).await
    }

    pub async fn ignore_user(
        &self,
        user_id: &str,
    ) -> Result<crate::app::user_profile::MatrixIgnoredUsersWriteResult, &'static str> {
        crate::app::user_profile::ignore_user(&self.client, user_id).await
    }

    pub async fn unignore_user(
        &self,
        user_id: &str,
    ) -> Result<crate::app::user_profile::MatrixIgnoredUsersWriteResult, &'static str> {
        crate::app::user_profile::unignore_user(&self.client, user_id).await
    }

    pub async fn search_user_directory(
        &self,
        term: &str,
        limit: Option<u64>,
    ) -> Result<crate::app::user_profile::MatrixUserDirectorySearchResult, &'static str> {
        crate::app::user_profile::search_user_directory(&self.client, term, limit).await
    }

    pub async fn search_messages(
        &self,
        term: &str,
        next_token: Option<&str>,
        rooms: Option<&[String]>,
        senders: Option<&[String]>,
        order: Option<&str>,
    ) -> Result<crate::app::search::MatrixMessageSearchResult, &'static str> {
        crate::app::search::search_messages(&self.client, term, next_token, rooms, senders, order)
            .await
    }

    pub async fn snapshot_push_rules(
        &self,
    ) -> Result<crate::app::notifications::MatrixPushRulesSnapshot, &'static str> {
        crate::app::notifications::snapshot_push_rules(&self.client).await
    }

    pub async fn set_push_rule_default(
        &self,
        encrypted: bool,
        one_to_one: bool,
        mode: &str,
    ) -> Result<crate::app::notifications::MatrixPushRulesWriteResult, &'static str> {
        crate::app::notifications::set_default_room_mode(&self.client, encrypted, one_to_one, mode)
            .await
    }

    pub async fn set_push_rule_mention(
        &self,
        rule_id: &str,
        enabled: bool,
    ) -> Result<crate::app::notifications::MatrixPushRulesWriteResult, &'static str> {
        crate::app::notifications::set_mention_enabled(&self.client, rule_id, enabled).await
    }

    pub async fn add_push_keyword(
        &self,
        keyword: &str,
    ) -> Result<crate::app::notifications::MatrixPushRulesWriteResult, &'static str> {
        crate::app::notifications::add_keyword(&self.client, keyword).await
    }

    pub async fn remove_push_keyword(
        &self,
        keyword: &str,
    ) -> Result<crate::app::notifications::MatrixPushRulesWriteResult, &'static str> {
        crate::app::notifications::remove_keyword(&self.client, keyword).await
    }

    pub async fn snapshot_room_notification(
        &self,
        room_id: &str,
    ) -> Result<crate::app::notifications::MatrixRoomNotificationSnapshot, &'static str> {
        crate::app::notifications::snapshot_room_notification(&self.client, room_id).await
    }

    pub async fn set_room_notification(
        &self,
        room_id: &str,
        mode: &str,
    ) -> Result<crate::app::notifications::MatrixRoomNotificationWriteResult, &'static str> {
        crate::app::notifications::set_room_notification(&self.client, room_id, mode).await
    }

    pub async fn snapshot_room_notifications(
        &self,
    ) -> Result<crate::app::notifications::MatrixRoomNotificationsSnapshot, &'static str> {
        crate::app::notifications::snapshot_room_notifications(&self.client).await
    }

    pub async fn snapshot_threepids(
        &self,
    ) -> Result<crate::app::user_profile::MatrixThreepidSnapshot, &'static str> {
        crate::app::user_profile::snapshot_threepids(&self.client).await
    }

    pub async fn delete_threepid_email(
        &self,
        address: &str,
    ) -> Result<crate::app::user_profile::MatrixThreepidWriteResult, &'static str> {
        crate::app::user_profile::delete_threepid_email(&self.client, address).await
    }

    pub async fn request_threepid_email_token(
        &self,
        email: &str,
    ) -> Result<crate::app::user_profile::MatrixThreepidEmailTokenResult, &'static str> {
        crate::app::user_profile::request_threepid_email_token(
            &self.client,
            email,
            &self.pending_threepid,
        )
        .await
    }

    pub async fn add_threepid_email(
        &self,
    ) -> Result<crate::app::user_profile::MatrixThreepidAddResult, &'static str> {
        crate::app::user_profile::add_threepid_email(&self.client, &self.pending_threepid).await
    }

    pub async fn add_threepid_email_password(
        &self,
        password: &str,
    ) -> Result<crate::app::user_profile::MatrixThreepidAddResult, &'static str> {
        crate::app::user_profile::add_threepid_email_password(
            &self.client,
            &self.pending_threepid,
            password,
        )
        .await
    }

    pub async fn upload_avatar(
        &self,
        payload: Vec<u8>,
        mime_type: &str,
    ) -> Result<crate::app::user_profile::MatrixUploadAvatarResult, &'static str> {
        crate::app::user_profile::upload_avatar(&self.client, payload, mime_type).await
    }

    pub async fn upload_content(
        &self,
        payload: Vec<u8>,
        mime_type: &str,
        filename: Option<&str>,
    ) -> Result<crate::app::media::MatrixUploadMediaResult, &'static str> {
        crate::app::media::upload_content(&self.client, payload, mime_type, filename).await
    }

    pub async fn send_room_attachment(
        &self,
        request: crate::app::send::SendRoomAttachmentRequest,
    ) -> Result<crate::app::send::MatrixSendRoomAttachmentResult, &'static str> {
        crate::app::send::send_room_attachment(&self.client, request).await
    }

    pub async fn download_plain_media(&self, content_uri: &str) -> Result<Vec<u8>, &'static str> {
        crate::app::media::download_plain_media(&self.client, content_uri).await
    }

    pub async fn thumbnail_plain_media(
        &self,
        content_uri: &str,
        width: u64,
        height: u64,
    ) -> Result<Vec<u8>, &'static str> {
        crate::app::media::thumbnail_plain_media(&self.client, content_uri, width, height).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes_content(marker: &str) -> SynaraRoomNotesContent {
        let mut content = SynaraRoomNotesContent::default();
        content.rooms.insert(
            format!("!{marker}:example.org"),
            super::super::SynaraRoomNotesRoom::default(),
        );
        content
    }

    #[test]
    fn invalid_room_id_is_privacy_safe_diagnostic() {
        let err = parse_room_id("not-a-room").unwrap_err();
        assert_eq!(err, "v-send.r-pack-read-invalid-room");
        assert!(!err.contains('@'));
        assert!(!err.contains('!'));
    }

    #[test]
    fn pending_notes_write_masks_the_pre_sync_cached_snapshot() {
        let now = Instant::now();
        let local = notes_content("local");
        let stale = notes_content("stale");
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.finish_mutation(Some(local.clone()), now);

        assert_eq!(
            state.project(Ok(stale), now + Duration::from_secs(1)),
            Ok(local)
        );
    }

    #[test]
    fn matching_post_write_sync_acknowledges_the_pending_projection() {
        let now = Instant::now();
        let local = notes_content("local");
        let stale_cache = notes_content("stale-cache");
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.finish_mutation(Some(local.clone()), now);
        state.observe_synchronized_event(Ok(local.clone()), now);

        assert_eq!(
            state.project(Ok(stale_cache), now + Duration::from_secs(1)),
            Ok(local)
        );
        assert!(state.pending.is_none());
    }

    #[test]
    fn differing_external_sync_is_bounded_before_the_lww_winner_surfaces() {
        let now = Instant::now();
        let local = notes_content("local");
        let external = notes_content("external");
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.finish_mutation(Some(local.clone()), now);
        state.observe_synchronized_event(Ok(external.clone()), now);

        assert_eq!(
            state.project(Ok(external.clone()), now + Duration::from_secs(1)),
            Ok(local)
        );
        assert_eq!(
            state.project(
                Ok(external.clone()),
                now + ROOM_NOTES_PENDING_PROJECTION_TTL
            ),
            Ok(external)
        );
    }

    #[test]
    fn sync_during_mutation_cannot_reinstate_pre_write_content() {
        let now = Instant::now();
        let local = notes_content("local");
        let stale = notes_content("stale");
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.observe_synchronized_event(Ok(stale.clone()), now);
        state.finish_mutation(Some(local.clone()), now);

        assert_eq!(
            state.project(Ok(stale), now + Duration::from_secs(1)),
            Ok(local)
        );
    }

    #[test]
    fn late_pre_put_sync_cannot_erase_a_successful_local_write() {
        let now = Instant::now();
        let local = notes_content("local");
        let stale = notes_content("pre-put");
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.finish_mutation(Some(local.clone()), now);
        // This callback can be delivered after the PUT even though its /sync
        // response was already in flight and still carries pre-write state.
        state.observe_synchronized_event(Ok(stale.clone()), now);

        assert_eq!(
            state.project(Ok(stale), now + Duration::from_secs(1)),
            Ok(local)
        );
        assert!(state.pending.is_some());
    }

    #[test]
    fn delayed_pre_put_sync_cannot_outlive_the_pending_write_overlay() {
        let now = Instant::now();
        let local = notes_content("local");
        let stale = notes_content("pre-put");
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.finish_mutation(Some(local.clone()), now);
        state.observe_synchronized_event(Ok(stale), now + Duration::from_secs(25));

        // At the pending TTL the SDK store has caught up to the successful
        // PUT. A delayed pre-PUT callback must not receive a fresh 30-second
        // lifetime of its own and hide that authoritative snapshot.
        assert_eq!(
            state.project(Ok(local.clone()), now + ROOM_NOTES_PENDING_PROJECTION_TTL),
            Ok(local)
        );
        assert!(state.pending.is_none());
        assert!(state.synchronized.is_none());
    }

    #[test]
    fn pending_notes_projection_is_bounded_and_restores_fail_closed_reads() {
        let now = Instant::now();
        let mut state = RoomNotesProjectionState::default();

        state.begin_mutation();
        state.finish_mutation(Some(notes_content("local")), now);

        assert_eq!(
            state.project(
                Err("v-timeline-room-notes-unsupported-version"),
                now + ROOM_NOTES_PENDING_PROJECTION_TTL
            ),
            Err("v-timeline-room-notes-unsupported-version")
        );
        assert!(state.pending.is_none());
    }

    #[test]
    fn preexisting_projection_cannot_hide_a_sync_delivered_during_mutation_after_ttl() {
        let now = Instant::now();
        let old = notes_content("old");
        let local = notes_content("local");
        let external = notes_content("external");
        let mut state = RoomNotesProjectionState::default();

        state.observe_synchronized_event(Ok(old), now);
        state.begin_mutation();
        state.observe_synchronized_event(Ok(external.clone()), now + Duration::from_secs(1));
        state.finish_mutation(Some(local.clone()), now + Duration::from_secs(2));

        assert_eq!(
            state.project(Ok(external.clone()), now + Duration::from_secs(3)),
            Ok(local)
        );
        assert_eq!(
            state.project(Ok(external.clone()), now + Duration::from_secs(32)),
            Ok(external.clone())
        );
        assert_eq!(
            state.project(Ok(external.clone()), now + Duration::from_secs(33)),
            Ok(external)
        );
    }
}
