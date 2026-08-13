//! Desktop bridge for `matrix_timeline_close` through `Core::command`.
//!
//! Core owns the live `NativeTimelineOwner` after the shell attaches it.
//! React still invokes `matrix_timeline_close`.

use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_CLOSE_COMMAND: &str = "matrix_timeline_close";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_close(
    core: &Core,
    stream_id: String,
) -> Result<bool, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_CLOSE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "streamId": stream_id }),
        })
        .await
        .map_err(map_timeline_close_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_close_response_error())
}

fn map_timeline_close_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "d0.3-timeline-close-failed",
        ),
    }
}

fn timeline_close_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "d0.3-timeline-close-failed",
    )
}
