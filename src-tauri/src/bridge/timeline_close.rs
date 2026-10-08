//! Desktop bridge for `matrix_timeline_close` through `Core::command`.
//!
//! Core owns the live `NativeTimelineOwner` after the shell attaches it.
//! React still invokes `matrix_timeline_close`.

use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_close(
    core: &Core,
    stream_id: String,
) -> Result<bool, MatrixAuthCommandError> {
    let response = core
        .timeline_close(synara_core::core_api::MatrixTimelineCloseRequest { stream_id })
        .await
        .map_err(map_timeline_close_core_error)?;
    Ok(response)
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
