//! Desktop bridge for `matrix_timeline_set_read_state` through `Core::command`.

use synara_core::app::timeline::{NativeTimelineReadAction, NativeTimelineReadStateReadback};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_SET_READ_STATE_COMMAND: &str = "matrix_timeline_set_read_state";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_set_read_state(
    core: &Core,
    stream_id: String,
    action: NativeTimelineReadAction,
) -> Result<NativeTimelineReadStateReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_SET_READ_STATE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "streamId": stream_id,
                "action": action,
            }),
        })
        .await
        .map_err(map_timeline_set_read_state_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_set_read_state_response_error())
}

fn map_timeline_set_read_state_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
            "v-timeline-view-not-open",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "v-timeline-view-read-state-failed",
        ),
    }
}

fn timeline_set_read_state_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "v-timeline-view-read-state-failed",
    )
}
