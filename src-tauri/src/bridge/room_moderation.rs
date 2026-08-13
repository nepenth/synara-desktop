//! Desktop bridges for room invite/kick/ban/unban through `Core::command`.

use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_invite(
    core: &Core,
    room_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: "matrix_room_invite".to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }),
    })
    .await
    .map_err(map_room_moderation_core_error)?;
    Ok(())
}

pub(crate) async fn room_kick(
    core: &Core,
    room_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: "matrix_room_kick".to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }),
    })
    .await
    .map_err(map_room_moderation_core_error)?;
    Ok(())
}

pub(crate) async fn room_ban(
    core: &Core,
    room_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: "matrix_room_ban".to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }),
    })
    .await
    .map_err(map_room_moderation_core_error)?;
    Ok(())
}

pub(crate) async fn room_set_power_level(
    core: &Core,
    room_id: String,
    user_id: String,
    power_level: i64,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: "matrix_room_set_power_level".to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "powerLevel": power_level,
        }),
    })
    .await
    .map_err(map_room_moderation_core_error)?;
    Ok(())
}

pub(crate) async fn room_unban(
    core: &Core,
    room_id: String,
    user_id: String,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: "matrix_room_unban".to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
        }),
    })
    .await
    .map_err(map_room_moderation_core_error)?;
    Ok(())
}

fn map_room_moderation_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms-members-moderation-invalid-room");
            let (code, message) = match diagnostic {
                "v-rooms-members-moderation-room-not-found" => (
                    "NotFound",
                    "The native Matrix moderation room is not available.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix member moderation request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms-members-moderation-ban-failed");
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix member moderation operation could not be completed.",
                diagnostic,
            )
        }
    }
}
