//! Native timeline: the live SDK owner, its view projection, and UTD state.
//!
//! [`NativeTimelineRegistry`] / [`NativeTimelineOwner`] in `live.rs` own every
//! production timeline: SDK timelines per room, thread and event focus, view
//! streams with their published rows, pagination and read state.
//!
//! The P5.1–P5.4 harness (a registry lifecycle, ordered-diff projection over
//! Synara `TimelineItem` DTOs, a pagination state machine and a focus
//! navigator) predates `live.rs` and is compiled for tests only. It does not
//! describe what the live owner does; read `live.rs` for that.
//!
//! Original design notes:
//! - `docs/matrix-rust-sdk/p5.1-timeline-registry.md`
//! - `docs/matrix-rust-sdk/p5.2-timeline-diffs.md`
//! - `docs/matrix-rust-sdk/p5.3-timeline-pagination.md`
//! - `docs/matrix-rust-sdk/p5.4-timeline-focus.md`
//! - `docs/matrix-rust-sdk/p5.10-utd.md`

mod actions;
mod composer;
#[cfg(test)]
mod delta;
mod error;
#[cfg(test)]
mod focus;
#[cfg(test)]
mod harness_tests;
mod live;
mod media;
mod native;
#[cfg(test)]
mod pagination;
mod pins;
#[cfg(test)]
mod projection;
mod reactions;
#[cfg(test)]
mod registry;
mod shield;
mod utd;
mod view;
mod view_emit;

pub use actions::{
    format_forwarded_media_body, format_forwarded_plain_body, should_attach_formatted_body,
    NativeTimelineActionKind, NativeTimelineActionReadback, NativeTimelineCallDeclineRequest,
    NativeTimelineEditTextRequest, NativeTimelineForwardMediaRequest,
    NativeTimelineForwardTextRequest, NativeTimelinePinRequest, NativeTimelinePollVoteRequest,
    NativeTimelineRedactRequest, NativeTimelineReportRequest,
    NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
};
pub use composer::{
    reply_draft_readback, ComposerDraftRegistry, NativeComposerClearReplyDraftRequest,
    NativeComposerReplyDraft, NativeComposerReplyDraftReadback,
    NativeComposerReplyDraftRoomRequest, NativeComposerSetReplyDraftRequest,
    NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
};

#[cfg(test)]
pub use delta::{TimelineDeltaBatch, TimelineDeltaOp, TimelineSnapshot};
pub use error::TimelineError;
#[cfg(test)]
pub use focus::{
    ContextWindow, FocusOpenOutcome, FocusOpenRequest, NavigationPhase, TimelineFocus, TimelineMode,
};
pub use live::{
    NativeAgentApprovalInboxCoverage, NativeAgentApprovalInboxItem,
    NativeAgentApprovalInboxSnapshot, NativeAgentApprovalInboxStatus, NativeRoomReadStateReadback,
    NativeTimelineOwner, NativeTimelineRegistry, UNCLAIMED_VIEW_STREAM_TTL,
};
pub use media::{
    is_timeline_media_handle, TimelineMediaRegistry, TimelineMediaSource,
    TIMELINE_MEDIA_HANDLE_PREFIX,
};
pub use native::{
    NativeAgentApprovalDecisionRequest, NativeAgentApprovalDecisionResult, NativeDecryptionState,
    NativeReactionMutation, NativeReactionMutationResult, NativeTimelineCloseRequest,
    NativeTimelineDirection, NativeTimelineEventLocation, NativeTimelineEventReadback,
    NativeTimelineEventRoute, NativeTimelineFollowLiveRequest, NativeTimelineItem,
    NativeTimelineJumpLatestRequest, NativeTimelineOpenPosition, NativeTimelineOpenReadback,
    NativeTimelineOpenRequest, NativeTimelineReaction, NativeTimelineReactionSender,
    NativeTimelineReadAction, NativeTimelineReadIntent, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineSnapshot, NativeTimelineTimestampToEventReadback,
    NativeTimelineViewPaginationRequest, NativeTimelineViewportHint, NativeUtdPhase,
    NativeUtdStatus, NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
};
#[cfg(test)]
pub use pagination::{
    DirectionStatus, PaginationDirection, PaginationOutcome, PaginationPhase, PaginationRequest,
    TimelinePagination,
};
pub use pins::{
    project_pinned_items, reactions_for_target, snapshot_pinned_events, NativePinnedEventsRequest,
    PinnedEventItem, PinnedEventsSnapshot, PINNED_EVENTS_SCHEMA_VERSION,
};
#[cfg(test)]
pub use projection::{reconstruct, TimelineProjection};
#[cfg(test)]
pub use registry::{TimelineEntry, TimelineKey, TimelineLifecycle, TimelineRegistry};
pub use shield::{
    project_encryption_shield, TimelineEncryptionShield, TimelineShieldCode, TimelineShieldTone,
    STRICT_SHIELDS,
};
pub use utd::{UtdEntry, UtdIndex, UtdPhase, UtdReasonCode, UtdUpdate, MAX_UTD_ENTRIES};
pub use view::{
    own_agent_approval_decision, project_event_row, project_event_row_base, project_formatted_body,
    project_forward_transport, project_local_echo_status, project_message_type_and_media,
    project_poll_answers, project_timeline_diffs, project_timeline_diffs_with_media,
    project_timeline_item, project_timeline_item_with_media, sanitize_incoming_html,
    ProjectedLocalEcho, TimelineAgentApprovalDecision, TimelineCallRow,
    TimelineEncryptedUnavailableRow, TimelineEventRowBase, TimelineForwardTransport,
    TimelineMediaHandle, TimelineMembershipRow, TimelineMessageRow, TimelineOtherRow,
    TimelinePageState, TimelinePaginationState, TimelinePollAnswer, TimelinePollRow,
    TimelineReaction, TimelineReactionSender, TimelineReadState, TimelineRedactedRow,
    TimelineReplyPreview, TimelineRoomActionAuthority, TimelineRowCapabilities, TimelineStateRow,
    TimelineThreadSummary, TimelineViewCapabilities, TimelineViewDeltaBatch, TimelineViewDeltaOp,
    TimelineViewMetadata, TimelineViewPosition, TimelineViewRow, TimelineViewSnapshot,
    NATIVE_TIMELINE_VIEW_UPDATED_EVENT, TIMELINE_VIEW_SCHEMA_VERSION,
};
pub use view_emit::{TimelineViewUpdateEmit, ViewDeltaEmitter};
