//! P5.1–P5.4 + P5.10 — Timeline registry, diffs, pagination, focus, UTD (harness).
//!
//! Per-room (and optional thread) timeline owners stamped with session
//! generation, pure ordered-diff projection over Synara [`TimelineItem`] DTOs,
//! a pagination state machine, live/unread/focused open navigation, and
//! UTD/decryption update propagation.
//! D0.3 adds a live SDK adapter for the product desktop session while retaining
//! these pure foundations. No dual backend and no session keys in errors.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p5.1-timeline-registry.md`
//! - `docs/matrix-rust-sdk/p5.2-timeline-diffs.md`
//! - `docs/matrix-rust-sdk/p5.3-timeline-pagination.md`
//! - `docs/matrix-rust-sdk/p5.4-timeline-focus.md`
//! - `docs/matrix-rust-sdk/p5.10-utd.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod actions;
mod composer;
mod delta;
mod error;
mod focus;
mod media;
mod native;
mod pagination;
mod projection;
mod registry;
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
    reply_draft_readback, ComposerDraftRegistry, NativeComposerReplyDraft,
    NativeComposerReplyDraftReadback, NativeComposerReplyDraftRoomRequest,
    NativeComposerSetReplyDraftRequest, NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
};

pub use delta::{TimelineDeltaBatch, TimelineDeltaOp, TimelineSnapshot};
pub use error::TimelineError;
pub use focus::{
    ContextWindow, FocusOpenOutcome, FocusOpenRequest, NavigationPhase, TimelineFocus, TimelineMode,
};
pub use media::{
    is_timeline_media_handle, TimelineMediaRegistry, TimelineMediaSource,
    TIMELINE_MEDIA_HANDLE_PREFIX,
};
pub use native::{
    NativeDecryptionState, NativeReactionMutation, NativeReactionMutationResult,
    NativeTimelineCloseRequest, NativeTimelineDirection, NativeTimelineEventReadback,
    NativeTimelineItem, NativeTimelineJumpLatestRequest, NativeTimelineOpenPosition,
    NativeTimelineOpenReadback, NativeTimelineOpenRequest, NativeTimelineReaction,
    NativeTimelineReactionSender, NativeTimelineReadAction, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineSnapshot, NativeTimelineViewPaginationRequest,
    NativeTimelineViewportHint, NativeUtdPhase, NativeUtdStatus,
    NATIVE_TIMELINE_OPEN_SCHEMA_VERSION, NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
};
pub use pagination::{
    DirectionStatus, PaginationDirection, PaginationOutcome, PaginationPhase, PaginationRequest,
    TimelinePagination,
};
pub use projection::{reconstruct, TimelineProjection};
pub use registry::{TimelineEntry, TimelineKey, TimelineLifecycle, TimelineRegistry};
pub use utd::{UtdEntry, UtdIndex, UtdPhase, UtdReasonCode, UtdUpdate, MAX_UTD_ENTRIES};
pub use view::{
    project_event_row, project_event_row_base, project_formatted_body,
    project_message_type_and_media, project_poll_answers, project_timeline_diffs,
    project_timeline_diffs_with_media, project_timeline_item, project_timeline_item_with_media,
    TimelineCallRow, TimelineEncryptedUnavailableRow, TimelineEventRowBase, TimelineMediaHandle,
    TimelineMembershipRow, TimelineMessageRow, TimelineOtherRow, TimelinePageState,
    TimelinePaginationState, TimelinePollAnswer, TimelinePollRow, TimelineReaction,
    TimelineReadState, TimelineRedactedRow, TimelineReplyPreview, TimelineRowCapabilities,
    TimelineStateRow, TimelineThreadSummary, TimelineViewCapabilities, TimelineViewDeltaBatch,
    TimelineViewDeltaOp, TimelineViewPosition, TimelineViewRow, TimelineViewSnapshot,
    NATIVE_TIMELINE_VIEW_UPDATED_EVENT, TIMELINE_VIEW_SCHEMA_VERSION,
};
pub use view_emit::{TimelineViewUpdateEmit, ViewDeltaEmitter};

/// Static marker for link / schema smoke.
pub const MATRIX_TIMELINE_MARKER: &str =
    "matrix-timeline-registry-p5.1+diffs-p5.2+pagination-p5.3+focus-p5.4+utd-p5.10";

/// Touch timeline registry + projection + pagination + focus + UTD paths so they remain linked.
pub fn matrix_timeline_markers() -> &'static str {
    let reg = TimelineRegistry::new(0);
    debug_assert!(reg.is_empty());
    debug_assert_eq!(reg.active_count(), 0);
    debug_assert_eq!(TimelineLifecycle::Live.as_str(), "live");
    let proj = TimelineProjection::new(0);
    debug_assert!(proj.is_empty());
    debug_assert_eq!(TimelineDeltaOp::Clear.op_name(), "clear");
    debug_assert_eq!(PaginationDirection::Backwards.as_str(), "backwards");
    debug_assert_eq!(PaginationPhase::Idle.as_str(), "idle");
    debug_assert_eq!(TimelineMode::Live.as_kind_str(), "live");
    debug_assert_eq!(NavigationPhase::Idle.as_str(), "idle");
    let utd = UtdIndex::new(0);
    debug_assert!(utd.is_empty());
    debug_assert_eq!(UtdReasonCode::MissingKeys.as_str(), "missing_keys");
    debug_assert_eq!(
        MATRIX_TIMELINE_MARKER,
        "matrix-timeline-registry-p5.1+diffs-p5.2+pagination-p5.3+focus-p5.4+utd-p5.10"
    );
    MATRIX_TIMELINE_MARKER
}
