//! Desktop bridge for `matrix_timeline_event_readback` through `Core::command`.

use synara_core::app::timeline::NativeTimelineEventReadback;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_EVENT_READBACK_COMMAND: &str = "matrix_timeline_event_readback";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_event_readback(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineEventReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_EVENT_READBACK_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "roomId": room_id, "eventId": event_id }),
        })
        .await
        .map_err(map_timeline_event_readback_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_event_readback_response_error())
}

fn map_timeline_event_readback_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
            "d0.3-timeline-invalid-room-id",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "v-crypto.6-event-open-failed",
        ),
    }
}

fn timeline_event_readback_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "v-crypto.6-event-open-failed",
    )
}
