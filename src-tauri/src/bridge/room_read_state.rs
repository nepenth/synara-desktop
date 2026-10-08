//! Desktop bridge for `matrix_room_set_read_state` through `Core::command`.

use synara_core::app::timeline::{NativeRoomReadStateReadback, NativeTimelineReadAction};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_set_read_state(
    core: &Core,
    room_id: String,
    action: NativeTimelineReadAction,
) -> Result<NativeRoomReadStateReadback, MatrixAuthCommandError> {
    let response = core
        .room_set_read_state(synara_core::core_api::MatrixRoomSetReadStateRequest {
            room_id,
            action,
        })
        .await
        .map_err(map_room_set_read_state_core_error)?;
    Ok(response)
}

fn map_room_set_read_state_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms-room-read-state-room-not-found");
            let (code, message) = match diagnostic {
                "v-rooms-room-read-state-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                "d0.3-timeline-invalid-room-id" | "p2-room-set-read-state-invalid-payload" => (
                    "InvalidRequest",
                    "The native Matrix room read request is invalid.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix room read request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms-room-read-state-mark-read-failed");
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix room read state is unavailable.",
                diagnostic,
            )
        }
    }
}
