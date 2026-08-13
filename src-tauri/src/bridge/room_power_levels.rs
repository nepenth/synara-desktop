//! Desktop bridges for bulk power-level writes through `Core::command`.

use synara_core::app::members::NativePowerLevelWriteResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_set_power_levels(
    core: &Core,
    room_id: String,
    content: serde_json::Value,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_room_set_power_levels",
        room_id,
        content,
        "m.room.power_levels",
    )
    .await
}

pub(crate) async fn room_set_power_level_tags(
    core: &Core,
    room_id: String,
    content: serde_json::Value,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_room_set_power_level_tags",
        room_id,
        content,
        "in.synara.room.power_level_tags",
    )
    .await
}

async fn dispatch(
    core: &Core,
    command: &str,
    room_id: String,
    content: serde_json::Value,
    event_type: &'static str,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "content": content,
            }),
        })
        .await
        .map_err(map_power_level_write_core_error)?;
    parse_power_level_write_result(response.payload, event_type)
}

fn parse_power_level_write_result(
    payload: serde_json::Value,
    event_type: &'static str,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        session_generation: u64,
        content: serde_json::Value,
        event_type: String,
        state_key: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| power_level_response_error())?;
    if wire.event_type != event_type || wire.state_key != "" {
        return Err(power_level_response_error());
    }
    Ok(NativePowerLevelWriteResult {
        status: "ok",
        room_id: wire.room_id,
        event_type,
        state_key: "",
        session_generation: wire.session_generation,
        content: wire.content,
    })
}

fn map_power_level_write_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-rooms-power-levels-send-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "StaleSessionGeneration",
            "The native Matrix session changed during the power-level write.",
            diagnostic,
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-rooms-power-levels-room-not-found" => (
                    "NotFound",
                    "The native Matrix power-level room is not available.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix power-level write request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix power-level write could not be completed.",
            diagnostic,
        ),
    }
}

fn power_level_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix power-level write could not be completed.",
        "v-rooms-power-levels-readback-malformed",
    )
}
