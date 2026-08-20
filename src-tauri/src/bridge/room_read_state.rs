//! Desktop bridge for `matrix_room_set_read_state` through `Core::command`.

use synara_core::app::timeline::NativeTimelineReadAction;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const ROOM_SET_READ_STATE_COMMAND: &str = "matrix_room_set_read_state";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_set_read_state(
    core: &Core,
    room_id: String,
    action: NativeTimelineReadAction,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: ROOM_SET_READ_STATE_COMMAND.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "roomId": room_id,
            "action": action,
        }),
    })
    .await
    .map_err(map_room_set_read_state_core_error)?;
    Ok(())
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
