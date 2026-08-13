//! Desktop bridges for timeline open/jump-latest through `Core::command`.
//!
//! The shell attaches a `NativeTimelineOwner` that already holds the view-delta
//! emit sink. React still invokes `matrix_timeline_open` and
//! `matrix_timeline_jump_latest`.

use synara_core::app::timeline::{NativeTimelineOpenPosition, NativeTimelineOpenReadback};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_OPEN_COMMAND: &str = "matrix_timeline_open";
const TIMELINE_JUMP_LATEST_COMMAND: &str = "matrix_timeline_jump_latest";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_open(
    core: &Core,
    room_id: String,
    position: NativeTimelineOpenPosition,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_OPEN_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "position": position,
            }),
        })
        .await
        .map_err(map_timeline_open_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_open_response_error())
}

pub(crate) async fn timeline_jump_latest(
    core: &Core,
    stream_id: String,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_JUMP_LATEST_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "streamId": stream_id }),
        })
        .await
        .map_err(map_timeline_open_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_open_response_error())
}

fn map_timeline_open_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-timeline-view-not-open") => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline request is invalid.",
                "v-timeline-view-not-open",
            ),
            Some("v-timeline-normal-room-not-found") => MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix timeline is not available.",
                "v-timeline-normal-room-not-found",
            ),
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline request is invalid.",
                "d0.3-timeline-invalid-room-id",
            ),
        },
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "d0.3-timeline-open-failed",
        ),
    }
}

fn timeline_open_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "d0.3-timeline-open-failed",
    )
}
