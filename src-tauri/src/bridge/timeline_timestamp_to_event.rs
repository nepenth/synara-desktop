//! Desktop bridge for `matrix_timeline_timestamp_to_event` through `Core::command`.

use synara_core::app::timeline::NativeTimelineTimestampToEventReadback;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_TIMESTAMP_TO_EVENT_COMMAND: &str = "matrix_timeline_timestamp_to_event";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_timestamp_to_event(
    core: &Core,
    room_id: String,
    timestamp_ms: u64,
) -> Result<NativeTimelineTimestampToEventReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_TIMESTAMP_TO_EVENT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "roomId": room_id, "timestampMs": timestamp_ms }),
        })
        .await
        .map_err(map_timeline_timestamp_to_event_core_error)?;
    serde_json::from_value(response.payload)
        .map_err(|_| timeline_timestamp_to_event_response_error())
}

fn map_timeline_timestamp_to_event_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
            "p2-timeline-timestamp-to-event-failed",
        ),
    }
}

fn timeline_timestamp_to_event_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "p2-timeline-timestamp-to-event-failed",
    )
}
