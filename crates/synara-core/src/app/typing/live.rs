//! Live V-ROOMS.4 typing projection and send ownership.

use std::sync::Arc;

use matrix_sdk::{
    event_handler::EventHandlerDropGuard, ruma::events::typing::SyncTypingEvent, Client, Room,
    RoomState,
};
use tokio::sync::Mutex;

use super::{NativeTypingSnapshot, TypingIndex, MAX_TYPING_USERS_PER_ROOM};

/// Owns live `m.typing` projection for one managed session.
pub struct NativeTypingOwner {
    index: Arc<Mutex<TypingIndex>>,
    _handler: EventHandlerDropGuard,
}

impl NativeTypingOwner {
    pub fn start(client: &Client, session_generation: u64) -> Result<Self, &'static str> {
        let own_user_id = client
            .user_id()
            .ok_or("v-rooms.4-typing-owner-user-missing")?
            .to_owned();
        let index = Arc::new(Mutex::new(TypingIndex::new(session_generation)));
        let index_for_handler = index.clone();
        let handle = client.add_event_handler(move |event: SyncTypingEvent, room: Room| {
            let index = index_for_handler.clone();
            let own_user_id = own_user_id.clone();
            async move {
                if room.state() != RoomState::Joined {
                    return;
                }
                let users: Vec<String> = event
                    .content
                    .user_ids
                    .into_iter()
                    .filter(|user_id| *user_id != own_user_id)
                    .take(MAX_TYPING_USERS_PER_ROOM)
                    .map(|user_id| user_id.to_string())
                    .collect();
                let mut index = index.lock().await;
                let _ = index.set_users(room.room_id().as_str(), users);
            }
        });
        Ok(Self {
            index,
            _handler: client.event_handler_drop_guard(handle),
        })
    }

    pub async fn snapshot(&self) -> NativeTypingSnapshot {
        let index = self.index.lock().await;
        NativeTypingSnapshot {
            session_generation: index.session_generation(),
            rooms: index.nonempty_snapshots(),
        }
    }
}

pub async fn set_typing_notice(
    client: &Client,
    room_id: &str,
    typing: bool,
) -> Result<(), &'static str> {
    let owned = matrix_sdk::ruma::OwnedRoomId::try_from(room_id)
        .map_err(|_| "v-rooms.4-typing-invalid-room")?;
    let room = client
        .get_room(&owned)
        .ok_or("v-rooms.4-typing-room-missing")?;
    if room.state() != RoomState::Joined {
        return Err("v-rooms.4-typing-room-not-joined");
    }
    room.typing_notice(typing)
        .await
        .map_err(|_| "v-rooms.4-typing-notice-failed")?;
    Ok(())
}
