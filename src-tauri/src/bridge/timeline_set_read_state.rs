//! Desktop bridge for `matrix_timeline_set_read_state` through `Core::command`.

use synara_core::app::timeline::{
    NativeTimelineReadAction, NativeTimelineReadIntent, NativeTimelineReadStateReadback,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_set_read_state(
    core: &Core,
    stream_id: String,
    action: NativeTimelineReadAction,
    intent: NativeTimelineReadIntent,
    observed_live_tail_event_id: Option<String>,
) -> Result<NativeTimelineReadStateReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_set_read_state(synara_core::core_api::MatrixTimelineSetReadStateRequest {
            stream_id,
            action,
            intent,
            observed_live_tail_event_id,
        })
        .await
        .map_err(map_timeline_set_read_state_core_error)?;
    Ok(response)
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
