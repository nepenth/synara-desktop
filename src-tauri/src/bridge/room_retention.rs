//! Desktop bridge for `matrix_room_retention` through `Core::command`.

use synara_core::app::room_profile::MatrixRoomRetentionSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_retention(
    core: &Core,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomRetentionSnapshot, MatrixAuthCommandError> {
    let response = core
        .room_retention(synara_core::core_api::MatrixRoomRetentionRequest {
            room_id,
            session_generation,
        })
        .await
        .map_err(map_room_retention_core_error)?;
    Ok(response)
}

fn map_room_retention_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-send.r-room-profile-retention-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-send.r-room-profile-retention-requires-session",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "Forbidden",
            "The native Matrix room retention session is stale.",
            "v-send.r-room-profile-retention-stale-generation",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-send.r-room-profile-retention-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix room retention request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room retention policy could not be read.",
            diagnostic,
        ),
    }
}
