//! Desktop bridges for per-room notification mode through `Core::command`.

use synara_core::app::notifications::{
    MatrixRoomNotificationSnapshot, MatrixRoomNotificationWriteResult,
    MatrixRoomNotificationsSnapshot,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_notification_snapshot(
    core: &Core,
    room_id: String,
) -> Result<MatrixRoomNotificationSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_room_notification_snapshot",
        serde_json::json!({ "roomId": room_id }),
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| room_notification_response_error())
}

pub(crate) async fn room_notification_set(
    core: &Core,
    room_id: String,
    mode: String,
) -> Result<MatrixRoomNotificationWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_room_notification_set",
        serde_json::json!({
            "roomId": room_id,
            "mode": mode,
        }),
    )
    .await?;
    parse_write(payload)
}

pub(crate) async fn room_notifications_snapshot(
    core: &Core,
) -> Result<MatrixRoomNotificationsSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_room_notifications_snapshot",
        serde_json::Value::Null,
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| room_notification_response_error())
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map(|response| response.payload)
    .map_err(map_room_notification_core_error)
}

fn parse_write(
    payload: serde_json::Value,
) -> Result<MatrixRoomNotificationWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    struct Wire {
        status: String,
    }
    let wire: Wire =
        serde_json::from_value(payload).map_err(|_| room_notification_response_error())?;
    if wire.status != "ok" {
        return Err(room_notification_response_error());
    }
    Ok(MatrixRoomNotificationWriteResult { status: "ok" })
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

fn room_notification_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native room-notification editor is unavailable.",
        "v-push.sdk-failed",
    )
}
