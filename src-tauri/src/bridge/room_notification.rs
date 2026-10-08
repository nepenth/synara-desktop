//! Desktop bridges for per-room notification mode through `Core::command`.

use synara_core::app::notifications::{
    MatrixRoomNotificationSnapshot, MatrixRoomNotificationWriteResult,
    MatrixRoomNotificationsSnapshot,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_notification_snapshot(
    core: &Core,
    room_id: String,
) -> Result<MatrixRoomNotificationSnapshot, MatrixAuthCommandError> {
    let payload = core
        .room_notification_snapshot(synara_core::core_api::MatrixRoomNotificationRoomRequest {
            room_id,
        })
        .await
        .map_err(map_room_notification_core_error)?;
    Ok(payload)
}

pub(crate) async fn room_notification_set(
    core: &Core,
    room_id: String,
    mode: String,
) -> Result<MatrixRoomNotificationWriteResult, MatrixAuthCommandError> {
    let payload = core
        .room_notification_set(synara_core::core_api::MatrixRoomNotificationSetRequest {
            room_id,
            mode,
        })
        .await
        .map_err(map_room_notification_core_error)?;
    Ok(payload)
}

pub(crate) async fn room_notifications_snapshot(
    core: &Core,
) -> Result<MatrixRoomNotificationsSnapshot, MatrixAuthCommandError> {
    let payload = core
        .room_notifications_snapshot()
        .await
        .map_err(map_room_notification_core_error)?;
    Ok(payload)
}

fn map_room_notification_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-push.sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native room-notification request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native room-notification editor is unavailable.",
            diagnostic,
        ),
    }
}
