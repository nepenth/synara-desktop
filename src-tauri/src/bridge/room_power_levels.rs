//! Desktop bridges for bulk power-level writes through `Core::command`.

use synara_core::app::members::NativePowerLevelWriteResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_set_power_levels(
    core: &Core,
    room_id: String,
    content: serde_json::Value,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    core.room_set_power_levels(synara_core::core_api::MatrixRoomSetPowerLevelStateRequest {
        room_id,
        content,
    })
    .await
    .map_err(map_power_level_write_core_error)
}

pub(crate) async fn room_set_power_level_tags(
    core: &Core,
    room_id: String,
    content: serde_json::Value,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    core.room_set_power_level_tags(synara_core::core_api::MatrixRoomSetPowerLevelStateRequest {
        room_id,
        content,
    })
    .await
    .map_err(map_power_level_write_core_error)
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
