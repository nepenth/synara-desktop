//! Desktop AppHandle adapter for the Core live timeline registry.
//!
//! `NativeTimelineRegistry` lives in synara-core. This file only maps the
//! Core emit sink onto the timeline views' IPC channels (`view_channels.rs`).

use std::sync::Arc;

use tauri::{AppHandle, Manager};

#[cfg(test)]
pub use synara_core::app::timeline::{
    NativeReactionMutation, NativeTimelineReaction, NativeTimelineRegistry,
};

#[cfg(test)]
pub use synara_core::app::timeline::NativeTimelineDirection;
pub use synara_core::app::timeline::{
    NativeReactionMutationResult, NativeTimelineCloseRequest, NativeTimelineEventReadback,
    NativeTimelineJumpLatestRequest, NativeTimelineOpenReadback, NativeTimelineOpenRequest,
    NativeTimelineOwner, NativeTimelineReadAction, NativeTimelineReadStateRequest,
    NativeTimelineViewPaginationRequest, TimelineViewUpdateEmit,
};

use super::view_channels::TimelineViewChannels;

/// Map a Tauri AppHandle onto the Core timeline view-delta sink.
///
/// Deltas go only to the channels the main window's timeline views opened.
/// A Tauri event would serialize every batch into a script for the webview
/// to evaluate, whether or not a view wanted it.
pub fn timeline_view_emit(app: AppHandle) -> TimelineViewUpdateEmit {
    let channels = app
        .try_state::<Arc<TimelineViewChannels>>()
        .map(|channels| Arc::clone(channels.inner()));
    Arc::new(move |batch| {
        if let Some(channels) = &channels {
            channels.deliver(&batch);
        }
    })
}
