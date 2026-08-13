//! Live im.ponies image-pack Client snapshot/set and subscribe owner.
//!
//! Shells supply the emit sink (desktop Tauri event / later iOS UniFFI).

use std::sync::Arc;

use matrix_sdk::event_handler::EventHandlerDropGuard;
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

use super::{
    is_image_pack_account_data_type, is_image_pack_room_state_type, pack_from_account_data,
    set_global_image_packs_content_guard, set_room_image_pack_content_guard,
    set_user_image_pack_content_guard, EmoteRoomsContent, NativeGlobalImagePacksSnapshot,
    NativeImagePack, NativeLaterSnapshot, NativeMDirectMutationResult, NativeMDirectSnapshot,
    NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot, NativeUserImagePackSnapshot,
    RoomNoteMoveDirection, SynaraLaterItem, SynaraRoomNoteItem, EMOTE_ROOMS_EVENT_TYPE,
    ROOM_EMOTES_EVENT_TYPE, USER_EMOTES_EVENT_TYPE,
};

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

        let emit_account = emit.clone();
        let account_handle = client.add_event_handler(move |event: AnyGlobalAccountDataEvent| {
            let emit = emit_account.clone();
            async move {
                let event_type = event.event_type().to_string();
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
        super::snapshot_room_notes(&self.client, self.session_generation).await
    }

    pub async fn room_notes_upsert(
        &self,
        item: SynaraRoomNoteItem,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        super::upsert_room_note_item(&self.client, self.session_generation, item).await
    }

    pub async fn room_notes_delete(
        &self,
        room_id: String,
        item_id: String,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        super::delete_room_note_item_live(&self.client, self.session_generation, room_id, item_id)
            .await
    }

    pub async fn room_notes_complete_todo(
        &self,
        room_id: String,
        item_id: String,
        completed: bool,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        super::complete_room_todo_item_live(
            &self.client,
            self.session_generation,
            room_id,
            item_id,
            completed,
            super::room_notes_now_ms(),
        )
        .await
    }

    pub async fn room_notes_move_todo(
        &self,
        room_id: String,
        item_id: String,
        direction: RoomNoteMoveDirection,
    ) -> Result<NativeRoomNotesSnapshot, &'static str> {
        super::move_room_todo_item_live(
            &self.client,
            self.session_generation,
            room_id,
            item_id,
            direction,
            super::room_notes_now_ms(),
        )
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_room_id_is_privacy_safe_diagnostic() {
        let err = parse_room_id("not-a-room").unwrap_err();
        assert_eq!(err, "v-send.r-pack-read-invalid-room");
        assert!(!err.contains('@'));
        assert!(!err.contains('!'));
    }
}
