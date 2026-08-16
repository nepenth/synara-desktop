//! Desktop bridge for `matrix_timeline_paginate` through `Core::command`.

use synara_core::app::timeline::{NativeTimelineDirection, TimelineViewSnapshot};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_PAGINATE_COMMAND: &str = "matrix_timeline_paginate";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_paginate(
    core: &Core,
    stream_id: String,
    direction: NativeTimelineDirection,
) -> Result<TimelineViewSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_PAGINATE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "streamId": stream_id,
                "direction": direction,
            }),
        })
        .await
        .map_err(map_timeline_paginate_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_paginate_response_error())
}

fn map_timeline_paginate_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
            "v-timeline-view-paginate-backwards-failed",
        ),
    }
}

fn timeline_paginate_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "v-timeline-view-paginate-backwards-failed",
    )
}
