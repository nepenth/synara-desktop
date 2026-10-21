//! Desktop adapter for the Core native timeline (live owner, views, UTD).
//!
//! SNC-P1-5c: the pure timeline logic now lives in the shared native core at
//! `crates/synara-core/src/app/timeline`. This module keeps every
//! `crate::matrix::timeline::*` path resolving with **identical behavior** by
//! re-exporting the core items plus the desktop `live.rs` AppHandle adapter
//! and the `view_channels.rs` delta channels it feeds.
//!
//! `product_commands.rs`, `tests.rs`, and `live_synapse_proof/` also stay here
//! (Platform commands = serial product lane; tests.rs = desktop suite via
//! `super::*`; live_synapse_proof = test-only network-proof harness).
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p5.1-timeline-registry.md`
//! - `docs/matrix-rust-sdk/p5.2-timeline-diffs.md`
//! - `docs/matrix-rust-sdk/p5.3-timeline-pagination.md`
//! - `docs/matrix-rust-sdk/p5.4-timeline-focus.md`
//! - `docs/matrix-rust-sdk/p5.10-utd.md`

mod live;
mod view_channels;

pub use view_channels::TimelineViewChannels;

#[cfg(test)]
pub use synara_core::app::timeline::{
    format_forwarded_media_body, format_forwarded_plain_body, NativeTimelineActionKind,
    NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
};

pub use synara_core::app::timeline::{
    is_timeline_media_handle, NativeComposerClearReplyDraftRequest,
    NativeComposerReplyDraftReadback, NativeComposerReplyDraftRoomRequest,
    NativeComposerSetReplyDraftRequest, NativePinnedEventsRequest, NativeTimelineActionReadback,
    NativeTimelineCallDeclineRequest, NativeTimelineEditTextRequest,
    NativeTimelineForwardMediaRequest, NativeTimelineForwardTextRequest, NativeTimelinePinRequest,
    NativeTimelinePollVoteRequest, NativeTimelineRedactRequest, NativeTimelineReportRequest,
    PinnedEventsSnapshot, TimelineMediaSource, TimelineViewSnapshot,
};

#[cfg(test)]
pub use synara_core::app::timeline::{
    project_formatted_body, project_message_type_and_media, project_poll_answers,
    reply_draft_readback, ComposerDraftRegistry, NativeComposerReplyDraft, TimelineEventRowBase,
    TimelineMediaRegistry, TimelineMessageRow, TimelinePollAnswer, TimelinePollRow,
    TimelineReplyPreview, TimelineRowCapabilities, TimelineThreadSummary, TimelineViewDeltaBatch,
    UtdIndex, UtdPhase, UtdReasonCode, UtdUpdate, TIMELINE_MEDIA_HANDLE_PREFIX,
    TIMELINE_VIEW_SCHEMA_VERSION,
};

#[cfg(test)]
pub use live::NativeTimelineDirection;

pub use live::{
    timeline_view_emit, NativeReactionMutationResult, NativeTimelineCloseRequest,
    NativeTimelineEventReadback, NativeTimelineJumpLatestRequest, NativeTimelineOpenReadback,
    NativeTimelineOpenRequest, NativeTimelineOwner, NativeTimelineReadAction,
    NativeTimelineReadStateRequest, NativeTimelineViewPaginationRequest,
};

#[cfg(test)]
mod live_synapse_proof;

#[cfg(test)]
mod tests;
