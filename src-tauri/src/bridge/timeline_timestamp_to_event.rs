//! Desktop bridge for `matrix_timeline_timestamp_to_event` through `Core::command`.

use synara_core::app::timeline::NativeTimelineTimestampToEventReadback;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_timestamp_to_event(
    core: &Core,
    room_id: String,
    timestamp_ms: u64,
) -> Result<NativeTimelineTimestampToEventReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_timestamp_to_event(
            synara_core::core_api::MatrixTimelineTimestampToEventRequest {
                room_id,
                timestamp_ms,
            },
        )
        .await
        .map_err(map_timeline_timestamp_to_event_core_error)?;
    Ok(response)
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
