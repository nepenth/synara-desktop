//! Desktop bridge for `matrix_timeline_set_read_state` through `Core::command`.

use serde::Serialize;
use synara_core::app::timeline::{
    NativeTimelineReadAction, NativeTimelineReadIntent, TimelineViewMetadata,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

/// A read action's outcome and the stream's metadata after it. The read
/// state also reaches the renderer as a delta batch, and the rows never
/// change with it, so none cross IPC here.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReadStateAck {
    pub action: NativeTimelineReadAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_sent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_event_id: Option<String>,
    pub view: TimelineViewMetadata,
}

pub(crate) async fn timeline_set_read_state(
    core: &Core,
    stream_id: String,
    action: NativeTimelineReadAction,
    intent: NativeTimelineReadIntent,
    observed_live_tail_event_id: Option<String>,
) -> Result<TimelineReadStateAck, MatrixAuthCommandError> {
    let response = core
        .timeline_set_read_state(synara_core::core_api::MatrixTimelineSetReadStateRequest {
            stream_id,
            action,
            intent,
            observed_live_tail_event_id,
        })
        .await
        .map_err(map_timeline_set_read_state_core_error)?;
    Ok(TimelineReadStateAck {
        action: response.action,
        receipt_sent: response.receipt_sent,
        acknowledged_event_id: response.acknowledged_event_id,
        view: response.snapshot.into(),
    })
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
