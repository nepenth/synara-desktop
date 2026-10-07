//! Core command adapters for messaging.

use super::*;

/// Exact React/Tauri envelope payload for `matrix_timeline_close`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineCloseRequest {
    pub(super) stream_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_open`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineOpenRequest {
    pub(super) room_id: String,
    pub(super) position: NativeTimelineOpenPosition,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineSnapshotRequest {
    pub(super) stream_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_jump_latest`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineJumpLatestRequest {
    pub(super) stream_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_event_readback`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineEventReadbackRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_timestamp_to_event`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineTimestampToEventRequest {
    pub(super) room_id: String,
    pub(super) timestamp_ms: u64,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_paginate`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelinePaginateRequest {
    pub(super) stream_id: String,
    pub(super) direction: NativeTimelineDirection,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_set_read_state`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineSetReadStateRequest {
    pub(super) stream_id: String,
    pub(super) action: NativeTimelineReadAction,
    pub(super) intent: NativeTimelineReadIntent,
    #[serde(default)]
    pub(super) observed_live_tail_event_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_follow_live`.
///
/// The renderer sends the tail event it painted at the visual bottom; Core
/// flips the stream position only when that event is still the
/// SDK-authoritative live tail. Unknown keys are rejected so the follow route
/// cannot grow identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineFollowLiveRequest {
    pub(super) stream_id: String,
    pub(super) observed_live_tail_event_id: String,
}

/// Exact React/Tauri envelope payload for reaction toggle/ensure.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineReactionKeyRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) key: String,
}

/// Exact React/Tauri envelope payload for `matrix_agent_approval_decide`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixAgentApprovalDecisionRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) action_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_reaction_redact`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixReactionRedactRequest {
    pub(super) room_id: String,
    pub(super) target_event_id: String,
    pub(super) reaction_event_id: String,
    pub(super) key: String,
}

/// Room id plus SDK transaction id for discard or retry of one local echo.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixLocalEchoRequest {
    pub(super) room_id: String,
    pub(super) transaction_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_send_text`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixSendTextRequest {
    pub(super) room_id: String,
    pub(super) body: String,
    #[serde(default)]
    pub(super) msg_type: Option<String>,
    #[serde(default)]
    pub(super) formatted_body: Option<String>,
    #[serde(default)]
    pub(super) mention_user_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(super) mention_room: Option<bool>,
    #[serde(default)]
    pub(super) reply_to: Option<String>,
    #[serde(default)]
    pub(super) thread_root: Option<String>,
    #[serde(default)]
    pub(super) txn_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_send_poll`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixSendPollRequest {
    pub(super) room_id: String,
    pub(super) question: String,
    pub(super) answers: Vec<String>,
    pub(super) max_selections: u32,
    #[serde(default)]
    pub(super) thread_root: Option<String>,
    #[serde(default)]
    pub(super) reply_to: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_poll_respond`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPollRespondRequest {
    pub(super) room_id: String,
    pub(super) poll_event_id: String,
    pub(super) answer_ids: Vec<String>,
}

/// Exact React/Tauri envelope payload for `matrix_edit_message`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixEditMessageRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) body: String,
    #[serde(default)]
    pub(super) msg_type: Option<String>,
    #[serde(default)]
    pub(super) formatted_body: Option<String>,
    #[serde(default)]
    pub(super) mention_user_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(super) mention_room: Option<bool>,
    #[serde(default)]
    pub(super) txn_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_edit_text`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineEditTextRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) body: String,
    #[serde(default)]
    pub(super) formatted_body: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_redact`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineRedactRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    #[serde(default)]
    pub(super) reason: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_report`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineReportRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    #[serde(default)]
    pub(super) reason: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_pin` / `matrix_timeline_unpin`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelinePinRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_pinned_events`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPinnedEventsRequest {
    pub(super) room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_poll_vote`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelinePollVoteRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    #[serde(default)]
    pub(super) answer_ids: Vec<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_call_decline`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineCallDeclineRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_forward_text`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineForwardTextRequest {
    pub(super) source_room_id: String,
    pub(super) event_id: String,
    pub(super) target_room_id: String,
    #[serde(default)]
    pub(super) as_quote: bool,
    pub(super) confirmed_encryption_downgrade: bool,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_forward_media`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTimelineForwardMediaRequest {
    pub(super) source_room_id: String,
    pub(super) event_id: String,
    pub(super) target_room_id: String,
    pub(super) confirmed_encryption_downgrade: bool,
}

/// Exact React/Tauri envelope payload for `matrix_composer_set_reply_draft`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixComposerSetReplyDraftRequest {
    pub(super) room_id: String,
    pub(super) event_id: String,
    #[serde(default)]
    pub(super) start_thread: bool,
}

/// Exact React/Tauri envelope payload for composer get reply-draft.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixComposerReplyDraftRoomRequest {
    pub(super) room_id: String,
    #[serde(default)]
    pub(super) thread_root_event_id: Option<String>,
}

/// Exact React/Tauri envelope payload for composer compare-and-clear.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixComposerClearReplyDraftRequest {
    pub(super) room_id: String,
    pub(super) expected_draft_revision: u64,
    #[serde(default)]
    pub(super) thread_root_event_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_thread_list`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixThreadListRequest {
    pub(super) room_id: String,
    pub(super) action: String,
}

pub(super) fn matrix_timeline_close(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineCloseRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-close-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-close-no-session")
        })?;
        let closed = owner.lock().await.close_view(NativeTimelineCloseRequest {
            stream_id: payload.stream_id,
        });
        Ok(serde_json::Value::Bool(closed))
    })
}

pub(super) fn matrix_timeline_open(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineOpenRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-open-invalid-payload"))?;
        let room_id = ruma::RoomId::parse(&payload.room_id).map_err(|_| {
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("d0.3-timeline-invalid-room-id")
        })?;
        if let Some(sync_owner) = state.sync_owner()? {
            sync_owner.subscribe_to_room(&room_id).await;
        }
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-open-no-session")
        })?;
        let readback: NativeTimelineOpenReadback = owner
            .open_at(NativeTimelineOpenRequest {
                room_id: payload.room_id,
                position: payload.position,
            })
            .await
            .map_err(timeline_open_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-open-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_jump_latest(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineJumpLatestRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-jump-latest-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-jump-latest-no-session")
        })?;
        let readback: NativeTimelineOpenReadback = owner
            .jump_latest(NativeTimelineJumpLatestRequest {
                stream_id: payload.stream_id,
            })
            .await
            .map_err(timeline_open_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-jump-latest-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-snapshot-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-snapshot-no-session")
        })?;
        let snapshot = owner
            .snapshot(&payload.stream_id)
            .await
            .map_err(timeline_open_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-timeline-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_retry_decryption(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-retry-decryption-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-retry-decryption-no-session")
        })?;
        let requested = owner
            .retry_decryption(&payload.stream_id)
            .await
            .map_err(timeline_open_owner_error)?;
        Ok(serde_json::Value::Bool(requested))
    })
}

pub(super) fn timeline_open_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id"
        | "v-timeline-view-not-open"
        | "v-timeline-normal-room-not-found"
        | "d0.3-timeline-room-not-found"
        | "v-timeline-thread-root-invalid"
        | "v-timeline-thread-room-not-found"
        | "v-timeline-thread-open-failed" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_timeline_event_readback(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineEventReadbackRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-timeline-event-readback-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-event-readback-no-session")
        })?;
        let readback: NativeTimelineEventReadback = owner
            .event_readback(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_event_readback_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-event-readback-serialization-failed"))
    })
}

pub(super) fn timeline_event_readback_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" | "v-crypto.6-invalid-event-id" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-crypto.6-event-room-not-found" | "d0.3-timeline-room-not-found" => {
            MatrixIpcErrorCategory::Forbidden
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_timeline_timestamp_to_event(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineTimestampToEventRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-timeline-timestamp-to-event-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-timestamp-to-event-no-session")
        })?;
        let readback: NativeTimelineTimestampToEventReadback = owner
            .timestamp_to_event(&payload.room_id, payload.timestamp_ms)
            .await
            .map_err(timeline_timestamp_to_event_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-timestamp-to-event-serialization-failed"))
    })
}

pub(super) fn timeline_timestamp_to_event_owner_error(
    diagnostic_id: &'static str,
) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" | "p2-timeline-timestamp-to-event-invalid-timestamp" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "d0.3-timeline-room-not-found" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_timeline_paginate(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePaginateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-paginate-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-paginate-no-session")
        })?;
        let snapshot: TimelineViewSnapshot = owner
            .paginate(NativeTimelineViewPaginationRequest {
                stream_id: payload.stream_id,
                direction: payload.direction,
            })
            .await
            .map_err(timeline_view_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-timeline-paginate-serialization-failed"))
    })
}

pub(super) fn timeline_view_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-timeline-view-not-open"
        | "v-timeline-read-observed-tail-required"
        | "v-timeline-read-observed-tail-invalid"
        | "v-timeline-read-observed-tail-unexpected"
        | "v-timeline-read-requires-live-view"
        | "v-timeline-read-mark-unread-requires-explicit-intent"
        | "v-timeline-send-thread-receipt-failed"
        | "v-timeline-send-read-markers-failed"
        | "v-timeline-follow-live-tail-required"
        | "v-timeline-follow-live-tail-invalid"
        | "v-timeline-follow-live-tail-not-loaded" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_timeline_set_read_state(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineSetReadStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-set-read-state-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-set-read-state-no-session")
        })?;
        let readback: NativeTimelineReadStateReadback = owner
            .set_read_state(NativeTimelineReadStateRequest {
                stream_id: payload.stream_id,
                action: payload.action,
                intent: payload.intent,
                observed_live_tail_event_id: payload.observed_live_tail_event_id,
            })
            .await
            .map_err(timeline_view_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-set-read-state-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_follow_live(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineFollowLiveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-follow-live-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-follow-live-no-session")
        })?;
        let snapshot: TimelineViewSnapshot = owner
            .follow_live_tail(NativeTimelineFollowLiveRequest {
                stream_id: payload.stream_id,
                observed_live_tail_event_id: payload.observed_live_tail_event_id,
            })
            .await
            .map_err(timeline_view_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-timeline-follow-live-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_reaction_toggle(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineReactionKeyRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-reaction-toggle-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-reaction-toggle-no-session")
        })?;
        let result: NativeReactionMutationResult = owner
            .toggle_reaction(&payload.room_id, &payload.event_id, &payload.key)
            .await
            .map_err(timeline_reaction_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-timeline-reaction-toggle-serialization-failed"))
    })
}

pub(super) fn matrix_reaction_ensure(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineReactionKeyRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-reaction-ensure-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-reaction-ensure-no-session")
        })?;
        let result: NativeReactionMutationResult = owner
            .ensure_reaction(&payload.room_id, &payload.event_id, &payload.key)
            .await
            .map_err(timeline_reaction_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-reaction-ensure-serialization-failed"))
    })
}

pub(super) fn matrix_agent_approvals_list(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Request {
            #[serde(default)]
            discovery_active: bool,
        }
        let payload: Request = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("agent-approval-inbox-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("agent-approval-no-session")
        })?;
        let result = owner
            .agent_approvals_list_with_discovery(payload.discovery_active)
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                    .with_diagnostic(diagnostic)
            })?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("agent-approval-inbox-serialization-failed"))
    })
}

pub(super) fn matrix_agent_approval_decide(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixAgentApprovalDecisionRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("agent-approval-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("agent-approval-no-session")
        })?;
        let result: NativeAgentApprovalDecisionResult = owner
            .decide_agent_approval(NativeAgentApprovalDecisionRequest {
                room_id: payload.room_id,
                event_id: payload.event_id,
                action_id: payload.action_id,
            })
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden).with_diagnostic(diagnostic)
            })?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("agent-approval-serialization-failed"))
    })
}

pub(super) fn matrix_reaction_redact(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixReactionRedactRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-reaction-redact-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-reaction-redact-no-session")
        })?;
        let result: NativeReactionMutationResult = owner
            .redact_reaction(
                &payload.room_id,
                &payload.target_event_id,
                &payload.reaction_event_id,
                &payload.key,
            )
            .await
            .map_err(timeline_reaction_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-reaction-redact-serialization-failed"))
    })
}

pub(super) fn timeline_reaction_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id"
        | "v-crypto.6-invalid-event-id"
        | "v-send.2-reaction-invalid-key"
        | "v-send.2-reaction-redact-annotation-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_send_text(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSendTextRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-send-text-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-send-text-no-session")
        })?;
        let result: MatrixSendTextResult = owner
            .send_text(
                payload.room_id,
                payload.body,
                payload.msg_type,
                payload.formatted_body,
                payload.mention_user_ids,
                payload.mention_room,
                payload.reply_to,
                payload.thread_root,
                payload.txn_id,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-send-text-serialization-failed"))
    })
}

pub(super) fn matrix_local_echo_discard(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLocalEchoRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-local-echo-discard-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-local-echo-discard-no-session")
        })?;
        let transaction_id = require_local_echo_transaction_id(&payload.transaction_id)?;
        let aborted = owner
            .abort_send(&payload.room_id, &transaction_id)
            .await
            .map_err(local_echo_owner_error)?;
        serde_json::to_value(serde_json::json!({
            "roomId": payload.room_id,
            "transactionId": transaction_id,
            "aborted": aborted,
        }))
        .map_err(|_| core_state_error("p2-local-echo-discard-serialization-failed"))
    })
}

pub(super) fn matrix_local_echo_retry(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLocalEchoRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-local-echo-retry-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-local-echo-retry-no-session")
        })?;
        let transaction_id = require_local_echo_transaction_id(&payload.transaction_id)?;
        owner
            .retry_send(&payload.room_id, &transaction_id)
            .await
            .map_err(local_echo_owner_error)?;
        serde_json::to_value(serde_json::json!({
            "roomId": payload.room_id,
            "transactionId": transaction_id,
            "status": "retrying",
        }))
        .map_err(|_| core_state_error("p2-local-echo-retry-serialization-failed"))
    })
}

fn require_local_echo_transaction_id(transaction_id: &str) -> Result<String, MatrixIpcError> {
    crate::app::send::parse_transaction_id(Some(transaction_id.to_owned()))
        .map_err(local_echo_owner_error)?
        .map(|txn_id| txn_id.to_string())
        .ok_or_else(|| local_echo_owner_error("d0.4-send-invalid-transaction-id"))
}

fn local_echo_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "d0.4-send-invalid-transaction-id"
        | "d0.4-send-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn send_text_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "d0.4-send-invalid-reply-event-id"
        | "d0.4-send-invalid-transaction-id"
        | "d0.4-send-text-payload-too-large"
        | "d0.4-send-room-not-found"
        | "v-send.4-invalid-message-type"
        | "v-send.4-invalid-mention-user-id"
        | "v-send.4-mention-user-id-too-long"
        | "v-send.4-too-many-mentions"
        | "v-send.5-invalid-thread-root-event-id"
        | "v-send.r-edit-invalid-event-id"
        | "v-send.r-edit-room-not-found"
        | "v-send-sticker-invalid-body"
        | "v-send-sticker-invalid-mxc"
        | "v-send-sticker-invalid-mimetype"
        | "v-send-sticker-room-not-found"
        | "v-send.3-poll-invalid-question"
        | "v-send.3-poll-invalid-answers"
        | "v-send.3-poll-invalid-event-id"
        | "v-send.3-poll-invalid-answer-ids"
        | "v-send.3-poll-room-not-found"
        | "p6.1-invalid-room-id"
        | "p6.1-empty-body"
        | "p6.1-body-too-large" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_send_poll(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSendPollRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-send-poll-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-send-poll-no-session")
        })?;
        let result: MatrixSendPollResult = owner
            .send_poll(
                payload.room_id,
                payload.question,
                payload.answers,
                payload.max_selections,
                payload.thread_root,
                payload.reply_to,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-send-poll-serialization-failed"))
    })
}

pub(super) fn matrix_poll_respond(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPollRespondRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-poll-respond-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-poll-respond-no-session")
        })?;
        let result: MatrixPollRespondResult = owner
            .poll_respond(payload.room_id, payload.poll_event_id, payload.answer_ids)
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-poll-respond-serialization-failed"))
    })
}

pub(super) fn matrix_edit_message(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixEditMessageRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-edit-message-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-edit-message-no-session")
        })?;
        let result: MatrixSendTextResult = owner
            .edit_message(
                payload.room_id,
                payload.event_id,
                payload.body,
                payload.msg_type,
                payload.formatted_body,
                payload.mention_user_ids,
                payload.mention_room,
                payload.txn_id,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-edit-message-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_edit_text(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineEditTextRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-edit-text-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-edit-text-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .edit_text(
                &payload.room_id,
                &payload.event_id,
                &payload.body,
                payload.formatted_body.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-edit-text-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_redact(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineRedactRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-redact-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-redact-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .redact_event(
                &payload.room_id,
                &payload.event_id,
                payload.reason.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-redact-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_report(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineReportRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-report-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-report-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .report(
                &payload.room_id,
                &payload.event_id,
                payload.reason.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-report-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_pin(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePinRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-pin-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-pin-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .pin_event(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-pin-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_unpin(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePinRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-unpin-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-unpin-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .unpin_event(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-unpin-serialization-failed"))
    })
}

pub(super) fn matrix_pinned_events(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPinnedEventsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-pinned-events-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-pinned-events-no-session")
        })?;
        let snapshot = owner
            .pinned_events_snapshot(&payload.room_id)
            .await
            .map_err(pinned_events_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-pinned-events-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_poll_vote(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePollVoteRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-poll-vote-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-poll-vote-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .poll_vote(&payload.room_id, &payload.event_id, payload.answer_ids)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-poll-vote-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_call_decline(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineCallDeclineRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-call-decline-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-call-decline-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .decline_call(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-call-decline-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_forward_text(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineForwardTextRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-forward-text-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-forward-text-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .forward_text(
                &payload.source_room_id,
                &payload.event_id,
                &payload.target_room_id,
                payload.as_quote,
                payload.confirmed_encryption_downgrade,
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-forward-text-serialization-failed"))
    })
}

pub(super) fn matrix_timeline_forward_media(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineForwardMediaRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-forward-media-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-forward-media-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .forward_media(
                &payload.source_room_id,
                &payload.event_id,
                &payload.target_room_id,
                payload.confirmed_encryption_downgrade,
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-forward-media-serialization-failed"))
    })
}

pub(super) fn matrix_composer_set_reply_draft(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixComposerSetReplyDraftRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-composer-set-reply-draft-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-composer-set-reply-draft-no-session")
        })?;
        let readback: NativeComposerReplyDraftReadback = owner
            .set_reply_draft(&payload.room_id, &payload.event_id, payload.start_thread)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-composer-set-reply-draft-serialization-failed"))
    })
}

pub(super) fn matrix_composer_clear_reply_draft(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixComposerClearReplyDraftRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-composer-clear-reply-draft-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-composer-clear-reply-draft-no-session")
        })?;
        let readback: NativeComposerReplyDraftReadback = owner
            .clear_reply_draft(
                &payload.room_id,
                payload.expected_draft_revision,
                payload.thread_root_event_id.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-composer-clear-reply-draft-serialization-failed"))
    })
}

pub(super) fn matrix_composer_get_reply_draft(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixComposerReplyDraftRoomRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-composer-get-reply-draft-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-composer-get-reply-draft-no-session")
        })?;
        let readback: NativeComposerReplyDraftReadback = owner
            .get_reply_draft(&payload.room_id, payload.thread_root_event_id.as_deref())
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-composer-get-reply-draft-serialization-failed"))
    })
}

pub(super) fn pinned_events_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    timeline_action_owner_error(diagnostic_id)
}

pub(super) fn matrix_thread_list(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixThreadListRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-thread-list-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-thread-list-no-session")
        })?;
        let snapshot: NativeThreadListSnapshot = owner
            .thread_list(&payload.room_id, &payload.action)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-thread-list-serialization-failed"))
    })
}

pub(super) fn timeline_action_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "d0.4-send-text-payload-too-large"
        | "v-timeline-edit-invalid-event-id"
        | "v-timeline-edit-empty-body"
        | "v-timeline-edit-room-not-found"
        | "v-timeline-redact-invalid-event-id"
        | "v-timeline-redact-room-not-found"
        | "v-timeline-report-invalid-event-id"
        | "v-timeline-report-room-not-found"
        | "v-timeline-pin-invalid-event-id"
        | "v-timeline-pin-room-not-found"
        | "v-timeline-unpin-invalid-event-id"
        | "v-timeline-unpin-room-not-found"
        | "v-timeline-pinned-room-not-found"
        | "v-timeline-pinned-cache-unavailable"
        | "v-timeline-pinned-subscribe-failed"
        | "v-timeline-poll-vote-invalid-event-id"
        | "v-timeline-poll-vote-room-not-found"
        | "v-timeline-call-decline-invalid-event-id"
        | "v-timeline-call-decline-room-not-found"
        | "v-timeline-call-decline-own-call"
        | "v-timeline-call-decline-bad-event-type"
        | "v-timeline-forward-invalid-event-id"
        | "v-timeline-forward-source-room-not-found"
        | "v-timeline-forward-target-room-not-found"
        | "v-timeline-forward-source-encryption-unavailable"
        | "v-timeline-forward-target-encryption-unavailable"
        | "v-timeline-forward-encryption-downgrade-not-confirmed"
        | "v-timeline-forward-event-unavailable"
        | "v-timeline-forward-event-decode-failed"
        | "v-timeline-forward-event-redacted"
        | "v-timeline-forward-unsupported-event"
        | "v-timeline-forward-media-invalid-event-id"
        | "v-timeline-forward-media-source-room-not-found"
        | "v-timeline-forward-media-target-room-not-found"
        | "v-timeline-forward-media-event-unavailable"
        | "v-timeline-forward-media-event-decode-failed"
        | "v-timeline-forward-media-event-redacted"
        | "v-timeline-forward-media-unsupported-event"
        | "v-timeline-reply-draft-invalid-event-id"
        | "v-timeline-reply-draft-room-not-found"
        | "v-timeline-reply-draft-event-unavailable"
        | "v-timeline-reply-draft-event-decode-failed"
        | "v-timeline-reply-draft-event-redacted"
        | "v-timeline-reply-draft-unsupported-event"
        | "v-thread-list-invalid-room-id"
        | "v-thread-list-room-not-found"
        | "v-thread-list-paginate-failed"
        | "v-thread-list-invalid-action" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}
