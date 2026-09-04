//! Desktop AppHandle adapter for the Core live timeline registry.
//!
//! `NativeTimelineRegistry` lives in synara-core. This file only maps the
//! existing `matrix-timeline-view-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

pub use synara_core::app::timeline::{
    NativeDecryptionState, NativeReactionMutation, NativeReactionMutationResult,
    NativeTimelineCloseRequest, NativeTimelineDirection, NativeTimelineEventReadback,
    NativeTimelineFollowLiveRequest, NativeTimelineItem, NativeTimelineJumpLatestRequest,
    NativeTimelineOpenPosition, NativeTimelineOpenReadback, NativeTimelineOpenRequest,
    NativeTimelineOwner, NativeTimelineReaction, NativeTimelineReactionSender,
    NativeTimelineReadAction, NativeTimelineReadIntent, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineRegistry, NativeTimelineSnapshot,
    NativeTimelineViewPaginationRequest, NativeTimelineViewportHint, NativeUtdPhase,
    NativeUtdStatus, TimelineViewUpdateEmit, NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
    NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS, NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
};

/// Map a Tauri AppHandle onto the Core timeline view-delta sink.
pub fn timeline_view_emit(app: AppHandle) -> TimelineViewUpdateEmit {
    Arc::new(move |batch| {
        let _ = app.emit(NATIVE_TIMELINE_VIEW_UPDATED_EVENT, batch);
    })
}
