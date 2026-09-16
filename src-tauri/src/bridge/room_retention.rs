//! Desktop bridge for `matrix_room_retention` through `Core::command`.

use synara_core::app::room_profile::MatrixRoomRetentionSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const COMMAND: &str = "matrix_room_retention";

pub(crate) async fn room_retention(
    core: &Core,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomRetentionSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: COMMAND.to_owned(),
            session_generation,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "sessionGeneration": session_generation,
            }),
        })
        .await
        .map_err(map_room_retention_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| retention_response_error())
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

fn retention_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room retention policy could not be read.",
        "v-send.r-room-profile-retention-sdk-failed",
    )
}
