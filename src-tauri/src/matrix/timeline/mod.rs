//! P5.1–P5.4 + P5.10 — Timeline registry, diffs, pagination, focus, UTD (src-tauri adapter).
//!
//! SNC-P1-5c: the pure timeline logic now lives in the shared native core at
//! `crates/synara-core/src/app/timeline`. This module keeps every
//! `crate::matrix::timeline::*` path resolving with **identical behavior** by
//! re-exporting the core items plus the desktop `live.rs` AppHandle adapter.
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

#![allow(dead_code)]
#![allow(unused_imports)]

mod live;

pub use synara_core::app::timeline::{
    format_forwarded_media_body, format_forwarded_plain_body, is_timeline_media_handle,
    project_event_row, project_event_row_base, project_formatted_body,
    project_message_type_and_media, project_poll_answers, project_timeline_diffs,
    project_timeline_diffs_with_media, project_timeline_item, project_timeline_item_with_media,
    reconstruct, reply_draft_readback, should_attach_formatted_body, ComposerDraftRegistry,
    ContextWindow, DirectionStatus, FocusOpenOutcome, FocusOpenRequest, NativeComposerReplyDraft,
    NativeComposerReplyDraftReadback, NativeComposerReplyDraftRoomRequest,
    NativeComposerSetReplyDraftRequest, NativeTimelineActionKind, NativeTimelineActionReadback,
    NativeTimelineCallDeclineRequest, NativeTimelineEditTextRequest,
    NativeTimelineForwardMediaRequest, NativeTimelineForwardTextRequest, NativeTimelinePinRequest,
    NativeTimelinePollVoteRequest, NativeTimelineRedactRequest, NativeTimelineReportRequest,
    NavigationPhase, PaginationDirection, PaginationOutcome, PaginationPhase, PaginationRequest,
    TimelineCallRow, TimelineDeltaBatch, TimelineDeltaOp, TimelineEncryptedUnavailableRow,
    TimelineEntry, TimelineError, TimelineEventRowBase, TimelineFocus, TimelineKey,
    TimelineLifecycle, TimelineMediaHandle, TimelineMediaRegistry, TimelineMediaSource,
    TimelineMembershipRow, TimelineMessageRow, TimelineMode, TimelineOtherRow, TimelinePageState,
    TimelinePagination, TimelinePaginationState, TimelinePollAnswer, TimelinePollRow,
    TimelineProjection, TimelineReaction, TimelineReadState, TimelineRedactedRow, TimelineRegistry,
    TimelineReplyPreview, TimelineRowCapabilities, TimelineSnapshot, TimelineStateRow,
    TimelineThreadSummary, TimelineViewCapabilities, TimelineViewDeltaBatch, TimelineViewDeltaOp,
    TimelineViewPosition, TimelineViewRow, TimelineViewSnapshot, UtdEntry, UtdIndex, UtdPhase,
    UtdReasonCode, UtdUpdate, MAX_UTD_ENTRIES, NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
    NATIVE_TIMELINE_ACTION_SCHEMA_VERSION, NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
    TIMELINE_MEDIA_HANDLE_PREFIX, TIMELINE_VIEW_SCHEMA_VERSION,
};

pub use live::{
    timeline_view_emit, NativeDecryptionState, NativeReactionMutation,
    NativeReactionMutationResult, NativeTimelineCloseRequest, NativeTimelineDirection,
    NativeTimelineEventReadback, NativeTimelineItem, NativeTimelineJumpLatestRequest,
    NativeTimelineOpenPosition, NativeTimelineOpenReadback, NativeTimelineOpenRequest,
    NativeTimelineReaction, NativeTimelineReactionSender, NativeTimelineReadAction,
    NativeTimelineReadStateReadback, NativeTimelineReadStateRequest, NativeTimelineRegistry,
    NativeTimelineSnapshot, NativeTimelineViewPaginationRequest, NativeTimelineViewportHint,
    NativeUtdPhase, NativeUtdStatus, NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
    NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
};

#[cfg(test)]
mod live_synapse_proof;

#[cfg(test)]
mod tests;

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
