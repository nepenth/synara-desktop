//! Live per-room notification mode via `Client::notification_settings()`.
//!
//! Wire modes reuse the global push-rules mapping. Failed errors never echo
//! room ids or modes.

use matrix_sdk::notification_settings::RoomNotificationMode;
use matrix_sdk::ruma::{OwnedRoomId, RoomId};
use matrix_sdk::Client;
use serde::{Deserialize, Serialize};

use super::push_rules::{mode_to_wire, parse_mode, MatrixPushRulesWriteResult};

const MAX_ROOM_OVERRIDES: usize = 1024;

/// One user-defined per-room notification override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomNotificationSnapshot {
    pub room_id: String,
    pub mode: String,
}

/// User-defined per-room overrides only. Rooms without an override are omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomNotificationsSnapshot {
    pub rooms: Vec<MatrixRoomNotificationSnapshot>,
}

pub type MatrixRoomNotificationWriteResult = MatrixPushRulesWriteResult;

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    RoomId::parse(room_id.trim()).map_err(|_| "v-push.invalid-room")
}

fn user_defined_mode_to_wire(mode: Option<RoomNotificationMode>) -> String {
    mode.map(mode_to_wire).unwrap_or("default").to_owned()
}

pub async fn snapshot_room_notification(
    client: &Client,
    room_id: &str,
) -> Result<MatrixRoomNotificationSnapshot, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let room_id = parse_room_id(room_id)?;
    let mode = client
        .notification_settings()
        .await
        .get_user_defined_room_notification_mode(&room_id)
        .await;
    Ok(MatrixRoomNotificationSnapshot {
        room_id: room_id.to_string(),
        mode: user_defined_mode_to_wire(mode),
    })
}

pub async fn set_room_notification(
    client: &Client,
    room_id: &str,
    mode: &str,
) -> Result<MatrixRoomNotificationWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let room_id = parse_room_id(room_id)?;
    let settings = client.notification_settings().await;
    match mode.trim() {
        "default" => settings
            .delete_user_defined_room_rules(&room_id)
            .await
            .map_err(|_| "v-push.sdk-failed")?,
        other => {
            let mode = parse_mode(other)?;
            settings
                .set_room_notification_mode(&room_id, mode)
                .await
                .map_err(|_| "v-push.sdk-failed")?;
        }
    }
    Ok(MatrixRoomNotificationWriteResult { status: "ok" })
}

pub async fn snapshot_room_notifications(
    client: &Client,
) -> Result<MatrixRoomNotificationsSnapshot, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let settings = client.notification_settings().await;
    let mut room_ids = settings.get_rooms_with_user_defined_rules(None).await;
    room_ids.sort();
    room_ids.dedup();
    room_ids.truncate(MAX_ROOM_OVERRIDES);
    let mut rooms = Vec::new();
    for room_id in room_ids {
        let Ok(parsed) = RoomId::parse(room_id.trim()) else {
            continue;
        };
        let Some(mode) = settings
            .get_user_defined_room_notification_mode(&parsed)
            .await
        else {
            continue;
        };
        rooms.push(MatrixRoomNotificationSnapshot {
            room_id: parsed.to_string(),
            mode: mode_to_wire(mode).to_owned(),
        });
    }
    Ok(MatrixRoomNotificationsSnapshot { rooms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_user_defined_wire_modes() {
        assert_eq!(user_defined_mode_to_wire(None), "default");
        assert_eq!(
            user_defined_mode_to_wire(Some(RoomNotificationMode::Mute)),
            "mute"
        );
        assert_eq!(
            parse_room_id("!r:example.org").unwrap().as_str(),
            "!r:example.org"
        );
        assert_eq!(
            parse_room_id("not-a-room").unwrap_err(),
            "v-push.invalid-room"
        );
        assert_eq!(parse_mode("default").unwrap_err(), "v-push.invalid-mode");
    }
}
