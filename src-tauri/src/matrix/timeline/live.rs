//! Desktop AppHandle adapter for the Core live timeline registry.
//!
//! `NativeTimelineRegistry` lives in synara-core. This file only maps the
//! existing `matrix-timeline-view-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

#[cfg(test)]
pub use synara_core::app::timeline::{
    NativeReactionMutation, NativeTimelineReaction, NativeTimelineRegistry,
};

#[cfg(test)]
pub use synara_core::app::timeline::NativeTimelineDirection;
pub use synara_core::app::timeline::{
    NativeReactionMutationResult, NativeTimelineCloseRequest, NativeTimelineEventReadback,
    NativeTimelineJumpLatestRequest, NativeTimelineOpenReadback, NativeTimelineOpenRequest,
    NativeTimelineOwner, NativeTimelineReadAction, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineViewPaginationRequest, TimelineViewUpdateEmit,
    NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
};

/// Map a Tauri AppHandle onto the Core timeline view-delta sink.
///
/// Deltas go to the main window only. A broadcast would also serialize every
/// batch into widget webviews, which never render the timeline.
pub fn timeline_view_emit(app: AppHandle) -> TimelineViewUpdateEmit {
    Arc::new(move |batch| {
        let _ = app.emit_to(
            crate::desktop::MAIN_WINDOW_LABEL,
            NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
            batch,
        );
    })
}
