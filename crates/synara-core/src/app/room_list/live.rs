//! Live D0.2 room-list snapshot projection.
//!
//! SDK room objects and vector diffs stop here. The Tauri boundary receives
//! only ordered room IDs and product-owned, privacy-safe summaries.

use std::sync::Arc;
use std::time::Duration;

use eyeball_im::VectorDiff;
use futures_util::StreamExt;
use matrix_sdk::notification_settings::RoomNotificationMode;
use matrix_sdk::{Room, RoomState};
use matrix_sdk_ui::room_list_service::filters;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::app::sync::SyncServiceOwner;
use crate::dto::{Membership, NotificationMode, RoomSummary};

/// Privacy-safe room-list wake-up. No room ids, names, tokens, or password.
/// iOS re-fetches via the existing snapshot command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRoomListUpdateSignal {
    pub session_generation: u64,
}

pub type RoomListUpdateEmit = Arc<dyn Fn(NativeRoomListUpdateSignal) + Send + Sync>;

/// Owns one joined-room entries stream for an attached SyncService.
pub struct NativeRoomListOwner {
    task: JoinHandle<()>,
}

impl NativeRoomListOwner {
    pub fn start(owner: &SyncServiceOwner, emit: RoomListUpdateEmit) -> Self {
        let service = owner.room_list_service();
        let session_generation = owner.session_generation();
        let task = tokio::spawn(async move {
            let Ok(list) = service.all_rooms().await else {
                return;
            };
            let (entries, controller) = list.entries_with_dynamic_adapters(usize::MAX);
            if !controller.set_filter(Box::new(filters::new_filter_joined())) {
                return;
            }
            futures_util::pin_mut!(entries);
            while let Some(diffs) = entries.next().await {
                if diffs.is_empty() {
                    continue;
                }
                emit(NativeRoomListUpdateSignal { session_generation });
            }
            drop(controller);
        });
        Self { task }
    }
}

impl Drop for NativeRoomListOwner {
    fn drop(&mut self) {
        self.task.abort();
    }
}

const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomListSnapshot {
    pub session_generation: u64,
    pub ordered_room_ids: Vec<String>,
    pub rooms: Vec<RoomSummary>,
}

pub async fn snapshot_from_sync_owner(
    owner: &SyncServiceOwner,
) -> Result<NativeRoomListSnapshot, &'static str> {
    let service = owner.room_list_service();
    let list = service
        .all_rooms()
        .await
        .map_err(|_| "d0.2-room-list-open-failed")?;
    let (entries, controller) = list.entries_with_dynamic_adapters(usize::MAX);
    if !controller.set_filter(Box::new(filters::new_filter_joined())) {
        return Err("d0.2-room-list-filter-failed");
    }

    futures_util::pin_mut!(entries);
    let diffs = tokio::time::timeout(SNAPSHOT_TIMEOUT, entries.next())
        .await
        .map_err(|_| "d0.2-room-list-snapshot-timeout")?
        .ok_or("d0.2-room-list-stream-ended")?;
    let values = diffs
        .into_iter()
        .find_map(|diff| match diff {
            VectorDiff::Reset { values } => Some(values),
            _ => None,
        })
        .ok_or("d0.2-room-list-reset-missing")?;

    let mut ordered_room_ids = Vec::with_capacity(values.len());
    let mut rooms = Vec::with_capacity(values.len());
    for item in values {
        ordered_room_ids.push(item.room_id().to_string());
        rooms.push(project_room(&item).await);
    }

    Ok(NativeRoomListSnapshot {
        session_generation: owner.session_generation(),
        ordered_room_ids,
        rooms,
    })
}

async fn project_room(room: &Room) -> RoomSummary {
    let counts = room.unread_notification_counts();
    let last_activity_ts = room
        .latest_event_timestamp()
        .map(|timestamp| timestamp.get().into())
        .or_else(|| room.recency_stamp().map(Into::into));
    let notification_mode = match room.cached_user_defined_notification_mode() {
        Some(mode) => Some(map_notification_mode(mode)),
        None => room.notification_mode().await.map(map_notification_mode),
    };
    // Room derefs to `BaseRoom`: `is_favourite`/`is_low_priority` read cached
    // `notable_tags` derived from the room's m.tag account data.
    RoomSummary {
        room_id: room.room_id().to_string(),
        name: room.cached_display_name().map(|name| name.to_string()),
        canonical_alias: room.canonical_alias().map(|alias| alias.to_string()),
        avatar_url: room.avatar_url().map(|uri| uri.to_string()),
        membership: membership(room.state()),
        is_direct: room.is_direct().await.unwrap_or(false),
        is_space: room.is_space(),
        is_call: room.is_call(),
        is_favorite: room.is_favourite(),
        is_low_priority: room.is_low_priority(),
        folder_id: None,
        is_encrypted: room
            .latest_encryption_state()
            .await
            .map(|state| state.is_encrypted())
            .unwrap_or(false),
        join_rule: None,
        unread_count: bounded_count(counts.notification_count),
        highlight_count: bounded_count(counts.highlight_count),
        marked_unread: room.is_marked_unread(),
        notification_mode,
        last_activity_ts,
        heroes: None,
        tombstone_successor_room_id: None,
    }
}

fn map_notification_mode(mode: RoomNotificationMode) -> NotificationMode {
    match mode {
        RoomNotificationMode::AllMessages => NotificationMode::All,
        RoomNotificationMode::MentionsAndKeywordsOnly => NotificationMode::Mentions,
        RoomNotificationMode::Mute => NotificationMode::Mute,
    }
}

fn membership(state: RoomState) -> Membership {
    match state {
        RoomState::Invited => Membership::Invite,
        RoomState::Joined => Membership::Join,
        RoomState::Knocked => Membership::Knock,
        RoomState::Left => Membership::Leave,
        RoomState::Banned => Membership::Ban,
    }
}

fn bounded_count(value: u64) -> u32 {
    value.min(u32::MAX.into()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_states_map_to_product_memberships() {
        assert_eq!(membership(RoomState::Joined), Membership::Join);
        assert_eq!(membership(RoomState::Invited), Membership::Invite);
        assert_eq!(membership(RoomState::Knocked), Membership::Knock);
        assert_eq!(membership(RoomState::Left), Membership::Leave);
        assert_eq!(membership(RoomState::Banned), Membership::Ban);
    }

    #[test]
    fn notification_modes_map_to_product_dto() {
        assert_eq!(
            map_notification_mode(RoomNotificationMode::AllMessages),
            NotificationMode::All
        );
        assert_eq!(
            map_notification_mode(RoomNotificationMode::MentionsAndKeywordsOnly),
            NotificationMode::Mentions
        );
        assert_eq!(
            map_notification_mode(RoomNotificationMode::Mute),
            NotificationMode::Mute
        );
    }

    #[test]
    fn unread_counts_are_bounded_for_ipc() {
        assert_eq!(bounded_count(7), 7);
        assert_eq!(bounded_count(u64::MAX), u32::MAX);
    }
}
