//! Live D0.2 room-list snapshot projection.
//!
//! SDK room objects and vector diffs stop here. The Tauri boundary receives
//! only ordered room IDs and product-owned, privacy-safe summaries.

use std::sync::Arc;
use std::time::Duration;

use eyeball_im::VectorDiff;
use futures_util::StreamExt;
use matrix_sdk::notification_settings::RoomNotificationMode;
use matrix_sdk::ruma::events::MessageLikeEventContent;
use matrix_sdk::{EncryptionState, Room, RoomState};
use matrix_sdk_ui::room_list_service::filters;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::app::room_list::counts::{room_unread_presentation, RoomUnreadMembership};
use crate::app::room_list::last_message::{
    last_message_preview_from_event_json, last_message_preview_from_event_json_str,
    last_message_preview_from_invite,
};
use crate::app::sync::SyncServiceOwner;
use crate::dto::{Membership, NotificationMode, RoomEncryptionStatus, RoomSummary};

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
const ROOM_LIST_SUBSCRIPTION_LIMIT: usize = 20;

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

    let viewport_room_ids = values
        .iter()
        .take(ROOM_LIST_SUBSCRIPTION_LIMIT)
        .map(|room| room.room_id().to_owned())
        .collect::<Vec<_>>();
    owner.subscribe_to_room_list(&viewport_room_ids).await;

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
    // `recency_stamp` is an opaque ordering value, not wall-clock time. Only
    // expose a timestamp when the SDK has an actual latest-event timestamp.
    let last_activity_ts = room
        .latest_event_timestamp()
        .map(|timestamp| timestamp.get().into());
    let notification_mode = match room.cached_user_defined_notification_mode() {
        Some(mode) => Some(map_notification_mode(mode)),
        None => room.notification_mode().await.map(map_notification_mode),
    };
    let membership = membership(room.state());
    let unread = room_unread_presentation(
        match membership {
            Membership::Invite => RoomUnreadMembership::Invited,
            _ => RoomUnreadMembership::Joined,
        },
        room.num_unread_messages(),
        room.num_unread_notifications()
            .max(counts.notification_count),
        room.num_unread_mentions().max(counts.highlight_count),
        room.is_marked_unread(),
    );
    // Room derefs to `BaseRoom`: `is_favourite`/`is_low_priority` read cached
    // `notable_tags` derived from the room's m.tag account data.
    let encryption_status = project_encryption_status(room.latest_encryption_state().await);
    RoomSummary {
        room_id: room.room_id().to_string(),
        name: room.cached_display_name().map(|name| name.to_string()),
        canonical_alias: room.canonical_alias().map(|alias| alias.to_string()),
        avatar_url: room.avatar_url().map(|uri| uri.to_string()),
        membership,
        is_direct: room.is_direct().await.unwrap_or(false),
        is_space: room.is_space(),
        is_call: room.is_call(),
        is_favorite: room.is_favourite(),
        is_low_priority: room.is_low_priority(),
        folder_id: None,
        encryption_status,
        join_rule: None,
        unread_count: bounded_count(unread.unread_count),
        highlight_count: bounded_count(room.num_unread_mentions().max(counts.highlight_count)),
        marked_unread: room.is_marked_unread(),
        notification_mode,
        last_activity_ts,
        last_message_preview: last_message_preview(room),
        heroes: None,
        tombstone_successor_room_id: None,
    }
}

fn project_encryption_status<E>(result: Result<EncryptionState, E>) -> RoomEncryptionStatus {
    match result {
        Ok(state) if state.is_unknown() => RoomEncryptionStatus::Unknown,
        Ok(state) if state.is_encrypted() => RoomEncryptionStatus::Encrypted,
        Ok(_) => RoomEncryptionStatus::NotEncrypted,
        Err(_) => RoomEncryptionStatus::Unknown,
    }
}

fn last_message_preview(room: &Room) -> Option<String> {
    use matrix_sdk::latest_events::LatestEventValue;
    match room.latest_event() {
        LatestEventValue::None => None,
        LatestEventValue::RemoteInvite { inviter, .. } => {
            last_message_preview_from_invite(inviter.as_ref().map(|id| id.as_str()))
        }
        LatestEventValue::Remote(event) => {
            last_message_preview_from_event_json_str(event.raw().json().get())
        }
        LatestEventValue::LocalIsSending(local)
        | LatestEventValue::LocalHasBeenSent { value: local, .. }
        | LatestEventValue::LocalCannotBeSent(local) => {
            let Ok(content) = local.content.deserialize() else {
                return None;
            };
            last_message_preview_from_event_json(&serde_json::json!({
                "type": content.event_type().to_string(),
                "content": content,
            }))
        }
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

    #[test]
    fn encryption_projection_preserves_unknown_and_errors_fail_closed() {
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::Encrypted)),
            RoomEncryptionStatus::Encrypted
        );
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::NotEncrypted)),
            RoomEncryptionStatus::NotEncrypted
        );
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::Unknown)),
            RoomEncryptionStatus::Unknown
        );
        assert_eq!(
            project_encryption_status::<()>(Err(())),
            RoomEncryptionStatus::Unknown
        );
    }
}
