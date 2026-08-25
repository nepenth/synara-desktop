use super::*;
use synara_core::app::timeline::NativeAgentApprovalDecisionResult;

#[tauri::command]
pub async fn matrix_timeline_open(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineOpenRequest,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_open::timeline_open(
        core.inner().as_ref(),
        request.room_id,
        request.position,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_close(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineCloseRequest,
) -> Result<bool, MatrixAuthCommandError> {
    crate::bridge::timeline_close::timeline_close(core.inner().as_ref(), request.stream_id).await
}

#[tauri::command]
pub async fn matrix_timeline_jump_latest(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineJumpLatestRequest,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_open::timeline_jump_latest(core.inner().as_ref(), request.stream_id)
        .await
}

#[tauri::command]
pub async fn matrix_timeline_event_readback(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineEventReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_event_readback::timeline_event_readback(
        core.inner().as_ref(),
        room_id,
        event_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_paginate(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineViewPaginationRequest,
) -> Result<crate::matrix::timeline::TimelineViewSnapshot, MatrixAuthCommandError> {
    crate::bridge::timeline_paginate::timeline_paginate(
        core.inner().as_ref(),
        request.stream_id,
        request.direction,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    stream_id: String,
) -> Result<crate::matrix::timeline::TimelineViewSnapshot, MatrixAuthCommandError> {
    crate::bridge::timeline_snapshot::timeline_snapshot(core.inner().as_ref(), stream_id).await
}

#[tauri::command]
pub async fn matrix_timeline_set_read_state(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineReadStateRequest,
) -> Result<NativeTimelineReadStateReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_set_read_state::timeline_set_read_state(
        core.inner().as_ref(),
        request.stream_id,
        request.action,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_reaction_toggle(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::reaction_toggle(
        core.inner().as_ref(),
        room_id,
        event_id,
        key,
    )
    .await
}

#[tauri::command]
pub async fn matrix_reaction_ensure(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::reaction_ensure(
        core.inner().as_ref(),
        room_id,
        event_id,
        key,
    )
    .await
}

#[tauri::command]
pub async fn matrix_agent_approval_decide(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
    action_id: String,
) -> Result<NativeAgentApprovalDecisionResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::agent_approval_decide(
        core.inner().as_ref(),
        room_id,
        event_id,
        action_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_reaction_redact(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    target_event_id: String,
    reaction_event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::reaction_redact(
        core.inner().as_ref(),
        room_id,
        target_event_id,
        reaction_event_id,
        key,
    )
    .await
}

#[tauri::command]
pub async fn matrix_composer_set_reply_draft(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeComposerSetReplyDraftRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_composer::composer_set_reply_draft(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.start_thread,
    )
    .await
}

#[tauri::command]
pub async fn matrix_composer_clear_reply_draft(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeComposerReplyDraftRoomRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_composer::composer_clear_reply_draft(
        core.inner().as_ref(),
        request.room_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_composer_get_reply_draft(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeComposerReplyDraftRoomRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_composer::composer_get_reply_draft(
        core.inner().as_ref(),
        request.room_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_edit_text(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineEditTextRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_edit_text(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.body,
        request.formatted_body,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_redact(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineRedactRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_redact(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.reason,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_forward_text(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineForwardTextRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_forward_text(
        core.inner().as_ref(),
        request.source_room_id,
        request.event_id,
        request.target_room_id,
        request.as_quote,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_forward_media(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineForwardMediaRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_forward_media(
        core.inner().as_ref(),
        request.source_room_id,
        request.event_id,
        request.target_room_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_report(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineReportRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_report(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.reason,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_pin(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelinePinRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_pin(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_unpin(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelinePinRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_unpin(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_poll_vote(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelinePollVoteRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_poll_vote(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.answer_ids,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_call_decline(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineCallDeclineRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_call_decline(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
    )
    .await
}

pub(super) fn map_timeline_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" => (
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
        ),
        "d0.3-timeline-room-not-found" | "d0.3-timeline-not-open" => {
            ("NotFound", "The native Matrix timeline is not available.")
        }
        _ => ("Unknown", "The native Matrix timeline is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_reaction_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let code = if diagnostic_id.contains("invalid") {
        "InvalidRequest"
    } else {
        "Unknown"
    };
    MatrixAuthCommandError::new(
        code,
        "The native Matrix reaction operation could not be completed.",
        diagnostic_id,
    )
}

pub(super) fn require_send_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        )
    })
}

pub(super) fn map_send_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "The native Matrix send request is invalid.",
        diagnostic_id,
    )
}

pub(super) fn parse_send_room_id(room_id: &str) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .parse()
        .map_err(|_| map_send_error("d0.4-send-invalid-room-id"))
}

pub(super) fn parse_reply_event_id(
    reply_to: Option<String>,
) -> Result<Option<OwnedEventId>, MatrixAuthCommandError> {
    reply_to
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| map_send_error("d0.4-send-invalid-reply-event-id"))
        })
        .transpose()
}

pub(super) fn parse_required_event_id(
    event_id: &str,
    diagnostic_id: &'static str,
) -> Result<OwnedEventId, MatrixAuthCommandError> {
    event_id
        .trim()
        .parse()
        .map_err(|_| map_timeline_action_error(diagnostic_id))
}

pub(super) fn map_timeline_action_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "The native Matrix timeline action request is invalid.",
        diagnostic_id,
    )
}
