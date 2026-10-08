//! Desktop bridge for `matrix_timeline_follow_live` through `Core::command`.

use synara_core::app::timeline::TimelineViewSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_follow_live(
    core: &Core,
    stream_id: String,
    observed_live_tail_event_id: String,
) -> Result<TimelineViewSnapshot, MatrixAuthCommandError> {
    let response = core
        .timeline_follow_live(synara_core::core_api::MatrixTimelineFollowLiveRequest {
            stream_id,
            observed_live_tail_event_id,
        })
        .await
        .map_err(map_timeline_follow_live_core_error)?;
    Ok(response)
}

fn map_timeline_follow_live_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-follow-live-tail-not-loaded"),
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "v-timeline-view-follow-live-failed",
        ),
    }
}
