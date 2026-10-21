//! Desktop bridge for `matrix_timeline_paginate` through `Core::command`.
//!
//! The page's rows reach the renderer as delta batches, so the reply carries
//! only the stream's metadata: no rows cross IPC twice.

use synara_core::app::timeline::{NativeTimelineDirection, TimelineViewMetadata};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_paginate(
    core: &Core,
    stream_id: String,
    direction: NativeTimelineDirection,
) -> Result<TimelineViewMetadata, MatrixAuthCommandError> {
    let response = core
        .timeline_paginate(synara_core::core_api::MatrixTimelinePaginateRequest {
            stream_id,
            direction,
        })
        .await
        .map_err(map_timeline_paginate_core_error)?;
    Ok(response.into())
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
