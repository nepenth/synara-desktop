//! Typed SharedCore operations and projections for timeline view.

use super::*;

/// Privacy-safe drained timeline view-delta summary. No row bodies or tokens.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewUpdateDto {
    pub schema_version: u32,
    pub session_generation: u64,
    pub stream_id: String,
    pub room_id: String,
    pub revision: u64,
    pub op_count: u32,
}

/// Static fail-closed timeline view-update poll error.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineViewUpdateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineViewUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineViewUpdateError {}

pub(super) fn timeline_view_poll_failed(
    code: &'static str,
    description: &'static str,
) -> TimelineViewUpdateError {
    TimelineViewUpdateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn timeline_view_update_dto(batch: TimelineViewDeltaBatch) -> TimelineViewUpdateDto {
    TimelineViewUpdateDto {
        schema_version: batch.schema_version,
        session_generation: batch.session_generation,
        stream_id: batch.stream_id,
        room_id: batch.room_id,
        revision: batch.revision,
        op_count: u32::try_from(batch.ops.len()).unwrap_or(u32::MAX),
    }
}

/// Requested open placement. Kind is a closed string; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineOpenPositionDto {
    pub kind: String,
    pub at_bottom: bool,
    pub restored_anchor_event_id: Option<String>,
    pub live_tail_event_id: Option<String>,
    pub updated_at_ms: Option<u64>,
    pub event_id: Option<String>,
}

/// Privacy-safe resolved view placement. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewPositionDto {
    pub kind: String,
    pub event_id: Option<String>,
}

/// Privacy-safe timeline snapshot. Identity/stream fields only; no token echo.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineSnapshotDto {
    pub schema_version: u32,
    pub session_generation: u64,
    pub room_id: String,
    pub revision: u64,
    pub position: TimelineViewPositionDto,
    pub pagination_backward: String,
    pub pagination_forward: String,
    pub visible_tail_event_id: Option<String>,
    pub receipt_tail_event_id: Option<String>,
    pub own_read_event_id: Option<String>,
    pub unread_anchor_event_id: Option<String>,
    pub is_marked_unread: bool,
    pub pinned_event_ids: Vec<String>,
    pub row_count: u32,
    pub mark_read: bool,
    pub mark_unread: bool,
    pub paginate_backward: bool,
    pub paginate_forward: bool,
    pub can_redact_own: bool,
    pub can_redact_other: bool,
    pub rows: Vec<TimelineViewRowDto>,
}

/// Privacy-safe timeline view row. Message text only; no media bytes or tokens.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewRowDto {
    pub kind: String,
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    /// Core-resolved display name with the Matrix user localpart as fallback.
    pub sender_name: String,
    /// Optional SDK-projected sender avatar. Metadata only and restricted to
    /// the Matrix `mxc://` content URI carried by the timeline profile.
    pub sender_avatar_url: Option<String>,
    pub body: String,
    pub origin_server_ts: u64,
    pub edited: bool,
    pub reply_to_event_id: Option<String>,
    pub reply_preview: Option<TimelineViewReplyPreviewDto>,
    pub thread_root_event_id: Option<String>,
    pub thread_summary: Option<TimelineViewThreadSummaryDto>,
    pub poll: Option<TimelineViewPollDto>,
    pub capabilities: Option<TimelineViewRowCapabilitiesDto>,
    pub decryption_state: Option<String>,
    pub message_type: Option<String>,
    /// Closed Core-owned dispatch route: `text` or `media`.
    pub forward_transport: Option<String>,
    pub formatted_body: Option<String>,
    pub agent_card_json: Option<String>,
    pub is_agent_approval: bool,
    pub media_filename: Option<String>,
    pub media_caption: Option<String>,
    pub reactions: Vec<TimelineViewReactionDto>,
    pub media_handle_id: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_width: Option<u32>,
    pub media_height: Option<u32>,
    pub media_duration_ms: Option<u64>,
    /// SDK authenticity shield for an event in an encrypted room. Absent when
    /// the event is trusted or no shield applies.
    pub encryption_shield: Option<TimelineViewEncryptionShieldDto>,
}

/// Closed authenticity shield. `tone` is `red` or `grey`; `code` is one of
/// `authenticity_not_guaranteed`, `unknown_device`, `unsigned_device`,
/// `unverified_identity`, `verification_violation`, `mismatched_sender` or
/// `sent_in_clear`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewEncryptionShieldDto {
    pub tone: String,
    pub code: String,
}

fn view_encryption_shield_dto(
    shield: crate::app::timeline::TimelineEncryptionShield,
) -> TimelineViewEncryptionShieldDto {
    TimelineViewEncryptionShieldDto {
        tone: shield.tone.as_str().to_owned(),
        code: shield.code.as_str().to_owned(),
    }
}

/// Privacy-safe reply preview projected by Core. No raw event content.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewReplyPreviewDto {
    pub event_id: String,
    pub sender_id: Option<String>,
    pub sender_name: String,
    pub body: String,
}

/// Privacy-safe thread summary projected by Core.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewThreadSummaryDto {
    pub root_event_id: String,
    pub reply_count: u32,
    pub latest_event_id: Option<String>,
}

/// One privacy-safe poll answer. Vote ownership is for the active account only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewPollAnswerDto {
    pub id: String,
    pub text: String,
    pub vote_count: u32,
    pub own: bool,
}

/// Privacy-safe poll presentation projected by Core. No voter identities.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewPollDto {
    pub question: String,
    pub closed: bool,
    pub max_selections: u32,
    pub answers: Vec<TimelineViewPollAnswerDto>,
}

/// Core-authoritative affordance gates for one timeline row.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewRowCapabilitiesDto {
    pub react: bool,
    pub reply: bool,
    pub edit: bool,
    pub redact: bool,
    pub report: bool,
    pub pin: bool,
    pub forward: bool,
    pub vote: bool,
    pub decline_call: bool,
}

/// Privacy-safe timeline open readback. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineOpenDto {
    pub schema_version: u32,
    pub stream_id: String,
    pub position: TimelineViewPositionDto,
    pub snapshot: TimelineSnapshotDto,
}

/// Privacy-safe single-event item. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineEventItemDto {
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    pub event_type: String,
    pub body: String,
    pub origin_server_ts: u64,
    pub decryption_state: Option<String>,
}

/// Privacy-safe single-event readback from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineEventReadbackDto {
    pub session_generation: u64,
    pub room_id: String,
    pub event_id: String,
    pub item: TimelineEventItemDto,
}

/// Privacy-safe read-state write ack. Reuses the S6 snapshot.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineReadStateDto {
    pub action: String,
    pub receipt_sent: Option<bool>,
    pub acknowledged_event_id: Option<String>,
    pub snapshot: TimelineSnapshotDto,
}

/// Static fail-closed timeline read-state error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineReadStateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineReadStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineReadStateError {}

/// Static fail-closed timeline error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineError {}

pub(super) fn timeline_failed(code: &'static str, description: &'static str) -> TimelineError {
    TimelineError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_open_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-open-no-session") => timeline_failed(
            TIMELINE_OPEN_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-normal-room-not-found") => timeline_failed(
            TIMELINE_ROOM_NOT_FOUND_CODE,
            TIMELINE_ROOM_NOT_FOUND_DESCRIPTION,
        ),
        Some("d0.3-timeline-room-not-found") => timeline_failed(
            "d0.3-timeline-room-not-found",
            TIMELINE_ROOM_NOT_FOUND_DESCRIPTION,
        ),
        Some("d0.3-timeline-invalid-room-id") => timeline_failed(
            TIMELINE_INVALID_ROOM_CODE,
            TIMELINE_INVALID_ROOM_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION),
    }
}

pub(super) fn map_timeline_close_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-close-no-session") => timeline_failed(
            TIMELINE_CLOSE_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        _ => timeline_failed(
            TIMELINE_CLOSE_FAILED_CODE,
            TIMELINE_CLOSE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn map_timeline_paginate_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-paginate-no-session") => timeline_failed(
            TIMELINE_PAGINATE_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(
            TIMELINE_PAGINATE_FAILED_CODE,
            TIMELINE_PAGINATE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn map_timeline_snapshot_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-snapshot-no-session") => timeline_failed(
            TIMELINE_SNAPSHOT_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION),
    }
}

pub(super) fn open_position_from_dto(
    position: TimelineOpenPositionDto,
) -> Result<NativeTimelineOpenPosition, TimelineError> {
    match position.kind.as_str() {
        "live" | "live_bottom" => Ok(NativeTimelineOpenPosition::LiveBottom),
        "unread" => Ok(NativeTimelineOpenPosition::Unread),
        "focused" => {
            let event_id = position
                .event_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
                })?;
            Ok(NativeTimelineOpenPosition::Focused { event_id })
        }
        "thread" => {
            let root_event_id = position
                .event_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
                })?;
            Ok(NativeTimelineOpenPosition::Thread { root_event_id })
        }
        "normal" => Ok(NativeTimelineOpenPosition::Normal {
            viewport: NativeTimelineViewportHint {
                at_bottom: position.at_bottom,
                restored_anchor_event_id: position.restored_anchor_event_id,
                live_tail_event_id: position.live_tail_event_id,
                updated_at_ms: position.updated_at_ms,
            },
        }),
        _ => Err(timeline_failed(
            TIMELINE_OPEN_FAILED_CODE,
            TIMELINE_OPEN_FAILED_DESCRIPTION,
        )),
    }
}

pub(super) fn paginate_direction(
    direction: &str,
) -> Result<NativeTimelineDirection, TimelineError> {
    match direction {
        "backwards" => Ok(NativeTimelineDirection::Backwards),
        "forwards" => Ok(NativeTimelineDirection::Forwards),
        _ => Err(timeline_failed(
            TIMELINE_PAGINATE_FAILED_CODE,
            TIMELINE_PAGINATE_FAILED_DESCRIPTION,
        )),
    }
}

pub(super) fn page_state_as_str(state: TimelinePageState) -> String {
    match state {
        TimelinePageState::Available => "available",
        TimelinePageState::Exhausted => "exhausted",
        TimelinePageState::Loading => "loading",
        TimelinePageState::Unavailable => "unavailable",
    }
    .to_owned()
}

pub(super) fn view_position_dto(position: TimelineViewPosition) -> TimelineViewPositionDto {
    match position {
        TimelineViewPosition::LiveBottom => TimelineViewPositionDto {
            kind: "live_bottom".to_owned(),
            event_id: None,
        },
        TimelineViewPosition::Unread { anchor_event_id } => TimelineViewPositionDto {
            kind: "unread".to_owned(),
            event_id: Some(anchor_event_id),
        },
        TimelineViewPosition::Focused { target_event_id } => TimelineViewPositionDto {
            kind: "focused".to_owned(),
            event_id: Some(target_event_id),
        },
        TimelineViewPosition::Thread { root_event_id } => TimelineViewPositionDto {
            kind: "thread".to_owned(),
            event_id: Some(root_event_id),
        },
        TimelineViewPosition::Restored { anchor_event_id } => TimelineViewPositionDto {
            kind: "restored".to_owned(),
            event_id: anchor_event_id,
        },
    }
}

pub(super) fn timeline_snapshot_dto(snapshot: TimelineViewSnapshot) -> TimelineSnapshotDto {
    TimelineSnapshotDto {
        schema_version: snapshot.schema_version,
        session_generation: snapshot.session_generation,
        room_id: snapshot.room_id,
        revision: snapshot.revision,
        position: view_position_dto(snapshot.position),
        pagination_backward: page_state_as_str(snapshot.pagination.backward),
        pagination_forward: page_state_as_str(snapshot.pagination.forward),
        visible_tail_event_id: snapshot.read_state.visible_tail_event_id,
        receipt_tail_event_id: snapshot.read_state.receipt_tail_event_id,
        own_read_event_id: snapshot.read_state.own_read_event_id,
        unread_anchor_event_id: snapshot.read_state.unread_anchor_event_id,
        is_marked_unread: snapshot.read_state.is_marked_unread,
        pinned_event_ids: snapshot.pinned_event_ids,
        row_count: u32::try_from(snapshot.rows.len()).unwrap_or(u32::MAX),
        mark_read: snapshot.capabilities.mark_read,
        mark_unread: snapshot.capabilities.mark_unread,
        paginate_backward: snapshot.capabilities.paginate_backward,
        paginate_forward: snapshot.capabilities.paginate_forward,
        can_redact_own: snapshot.capabilities.can_redact_own,
        can_redact_other: snapshot.capabilities.can_redact_other,
        rows: snapshot
            .rows
            .into_iter()
            .map(timeline_view_row_dto)
            .collect(),
    }
}

pub(super) fn view_reply_preview_dto(preview: TimelineReplyPreview) -> TimelineViewReplyPreviewDto {
    TimelineViewReplyPreviewDto {
        event_id: preview.event_id,
        sender_id: preview.sender_id,
        sender_name: preview.sender_name,
        body: preview.body,
    }
}

pub(super) fn view_thread_summary_dto(
    summary: TimelineThreadSummary,
) -> TimelineViewThreadSummaryDto {
    TimelineViewThreadSummaryDto {
        root_event_id: summary.root_event_id,
        reply_count: summary.reply_count,
        latest_event_id: summary.latest_event_id,
    }
}

pub(super) fn view_poll_answer_dto(answer: TimelinePollAnswer) -> TimelineViewPollAnswerDto {
    TimelineViewPollAnswerDto {
        id: answer.id,
        text: answer.text,
        vote_count: answer.vote_count,
        own: answer.own,
    }
}

pub(super) fn view_poll_dto(poll: &TimelinePollRow) -> TimelineViewPollDto {
    TimelineViewPollDto {
        question: poll.question.clone(),
        closed: poll.closed,
        max_selections: poll.max_selections,
        answers: poll
            .answers
            .iter()
            .cloned()
            .map(view_poll_answer_dto)
            .collect(),
    }
}

pub(super) fn view_row_capabilities_dto(
    capabilities: TimelineRowCapabilities,
) -> TimelineViewRowCapabilitiesDto {
    TimelineViewRowCapabilitiesDto {
        react: capabilities.react,
        reply: capabilities.reply,
        edit: capabilities.edit,
        redact: capabilities.redact,
        report: capabilities.report,
        pin: capabilities.pin,
        forward: capabilities.forward,
        vote: capabilities.vote,
        decline_call: capabilities.decline_call,
    }
}

pub(super) fn timeline_view_row_dto(row: TimelineViewRow) -> TimelineViewRowDto {
    match row {
        TimelineViewRow::Message(message) => {
            let (media_handle_id, media_mime_type, media_width, media_height, media_duration_ms) =
                view_media_fields(message.media);
            let reply_preview = message.reply.map(view_reply_preview_dto);
            let reply_to_event_id = reply_preview
                .as_ref()
                .map(|preview| preview.event_id.clone());
            let thread_summary = message.thread.map(view_thread_summary_dto);
            let capabilities = Some(view_row_capabilities_dto(message.event.capabilities));
            TimelineViewRowDto {
                kind: "message".to_owned(),
                item_id: message.event.item_id,
                event_id: message.event.event_id.unwrap_or_default(),
                sender: message.event.sender_id,
                sender_name: message.event.sender_name,
                sender_avatar_url: message.event.sender_avatar_url,
                body: message.body,
                origin_server_ts: message.event.origin_server_ts,
                edited: message.edited,
                reply_to_event_id,
                reply_preview,
                thread_root_event_id: message.thread_root,
                thread_summary,
                poll: None,
                capabilities,
                decryption_state: None,
                message_type: message.message_type,
                forward_transport: message
                    .forward_transport
                    .map(|transport| transport.as_str().to_owned()),
                formatted_body: message.formatted_body,
                agent_card_json: message.agent_card_json,
                is_agent_approval: message.is_agent_approval,
                media_filename: message.media_filename,
                media_caption: message.media_caption,
                reactions: view_reaction_dtos(message.reactions),
                media_handle_id,
                media_mime_type,
                media_width,
                media_height,
                media_duration_ms,
                encryption_shield: message
                    .event
                    .encryption_shield
                    .map(view_encryption_shield_dto),
            }
        }
        TimelineViewRow::Sticker {
            event,
            media,
            forward_transport,
            reply,
            thread_root,
            thread,
            reactions,
        } => {
            let (media_handle_id, media_mime_type, media_width, media_height, media_duration_ms) =
                view_media_fields(Some(media));
            let capabilities = Some(view_row_capabilities_dto(event.capabilities));
            let reply_preview = reply.map(view_reply_preview_dto);
            let reply_to_event_id = reply_preview
                .as_ref()
                .map(|preview| preview.event_id.clone());
            TimelineViewRowDto {
                kind: "sticker".to_owned(),
                item_id: event.item_id,
                event_id: event.event_id.unwrap_or_default(),
                sender: event.sender_id,
                sender_name: event.sender_name,
                sender_avatar_url: event.sender_avatar_url,
                body: String::new(),
                origin_server_ts: event.origin_server_ts,
                edited: false,
                reply_to_event_id,
                reply_preview,
                thread_root_event_id: thread_root,
                thread_summary: thread.map(view_thread_summary_dto),
                poll: None,
                capabilities,
                decryption_state: None,
                message_type: Some("m.sticker".to_owned()),
                forward_transport: Some(forward_transport.as_str().to_owned()),
                formatted_body: None,
                agent_card_json: None,
                is_agent_approval: false,
                media_filename: None,
                media_caption: None,
                reactions: view_reaction_dtos(reactions),
                media_handle_id,
                media_mime_type,
                media_width,
                media_height,
                media_duration_ms,
                encryption_shield: event.encryption_shield.map(view_encryption_shield_dto),
            }
        }
        TimelineViewRow::Poll(poll) => {
            let poll_dto = view_poll_dto(&poll);
            let capabilities = view_row_capabilities_dto(poll.event.capabilities);
            let reply_preview = poll.reply.map(view_reply_preview_dto);
            let reply_to_event_id = reply_preview
                .as_ref()
                .map(|preview| preview.event_id.clone());
            TimelineViewRowDto {
                kind: "poll".to_owned(),
                item_id: poll.event.item_id,
                event_id: poll.event.event_id.unwrap_or_default(),
                sender: poll.event.sender_id,
                sender_name: poll.event.sender_name,
                sender_avatar_url: poll.event.sender_avatar_url,
                body: poll.question,
                origin_server_ts: poll.event.origin_server_ts,
                edited: false,
                reply_to_event_id,
                reply_preview,
                thread_root_event_id: poll.thread_root,
                thread_summary: poll.thread.map(view_thread_summary_dto),
                poll: Some(poll_dto),
                capabilities: Some(capabilities),
                decryption_state: None,
                message_type: None,
                forward_transport: None,
                formatted_body: None,
                agent_card_json: None,
                is_agent_approval: false,
                media_filename: None,
                media_caption: None,
                reactions: view_reaction_dtos(poll.reactions),
                media_handle_id: None,
                media_mime_type: None,
                media_width: None,
                media_height: None,
                media_duration_ms: None,
                encryption_shield: poll.event.encryption_shield.map(view_encryption_shield_dto),
            }
        }
        TimelineViewRow::Membership(membership) => TimelineViewRowDto {
            kind: "membership".to_owned(),
            item_id: membership.event.item_id,
            event_id: membership.event.event_id.unwrap_or_default(),
            sender: membership.event.sender_id,
            sender_name: membership.event.sender_name,
            sender_avatar_url: membership.event.sender_avatar_url,
            body: membership.summary,
            origin_server_ts: membership.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(membership.event.capabilities)),
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
            encryption_shield: None,
        },
        TimelineViewRow::State(state) => TimelineViewRowDto {
            kind: "state".to_owned(),
            item_id: state.event.item_id,
            event_id: state.event.event_id.unwrap_or_default(),
            sender: state.event.sender_id,
            sender_name: state.event.sender_name,
            sender_avatar_url: state.event.sender_avatar_url,
            body: state.summary,
            origin_server_ts: state.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(state.event.capabilities)),
            decryption_state: None,
            message_type: Some(state.state_type),
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
            encryption_shield: None,
        },
        TimelineViewRow::Call(call) => TimelineViewRowDto {
            kind: "call".to_owned(),
            item_id: call.event.item_id,
            event_id: call.event.event_id.unwrap_or_default(),
            sender: call.event.sender_id,
            sender_name: call.event.sender_name,
            sender_avatar_url: call.event.sender_avatar_url,
            body: call.call_kind,
            origin_server_ts: call.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(call.event.capabilities)),
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
            encryption_shield: None,
        },
        TimelineViewRow::Redacted(redacted) => TimelineViewRowDto {
            kind: "redacted".to_owned(),
            item_id: redacted.event.item_id,
            event_id: redacted.event.event_id.unwrap_or_default(),
            sender: redacted.event.sender_id,
            sender_name: redacted.event.sender_name,
            sender_avatar_url: redacted.event.sender_avatar_url,
            body: redacted.summary,
            origin_server_ts: redacted.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(redacted.event.capabilities)),
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
            encryption_shield: None,
        },
        TimelineViewRow::EncryptedUnavailable(encrypted) => TimelineViewRowDto {
            kind: "encrypted".to_owned(),
            item_id: encrypted.event.item_id,
            event_id: encrypted.event.event_id.unwrap_or_default(),
            sender: encrypted.event.sender_id,
            sender_name: encrypted.event.sender_name,
            sender_avatar_url: encrypted.event.sender_avatar_url,
            body: encrypted.reason_code.clone(),
            origin_server_ts: encrypted.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(encrypted.event.capabilities)),
            decryption_state: Some(encrypted.reason_code),
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
            encryption_shield: None,
        },
        TimelineViewRow::Other(other) => {
            let (sender, sender_name, sender_avatar_url, origin_server_ts, capabilities) = other
                .event
                .map(|event| {
                    (
                        event.sender_id,
                        event.sender_name,
                        event.sender_avatar_url,
                        event.origin_server_ts,
                        Some(view_row_capabilities_dto(event.capabilities)),
                    )
                })
                .unwrap_or_else(|| (String::new(), String::new(), None, 0, None));
            TimelineViewRowDto {
                kind: "other".to_owned(),
                item_id: other.item_id,
                event_id: other.event_id.unwrap_or_default(),
                sender,
                sender_name,
                sender_avatar_url,
                body: other.summary,
                origin_server_ts,
                edited: false,
                reply_to_event_id: None,
                reply_preview: None,
                thread_root_event_id: None,
                thread_summary: None,
                poll: None,
                capabilities,
                decryption_state: None,
                message_type: other.event_type,
                forward_transport: other
                    .forward_transport
                    .map(|transport| transport.as_str().to_owned()),
                formatted_body: None,
                agent_card_json: None,
                is_agent_approval: false,
                media_filename: None,
                media_caption: None,
                reactions: Vec::new(),
                media_handle_id: None,
                media_mime_type: None,
                media_width: None,
                media_height: None,
                media_duration_ms: None,
                encryption_shield: None,
            }
        }
        TimelineViewRow::DateSeparator {
            item_id,
            timestamp_ms,
        } => TimelineViewRowDto {
            kind: "date_separator".to_owned(),
            item_id,
            event_id: String::new(),
            sender: String::new(),
            sender_name: String::new(),
            sender_avatar_url: None,
            body: String::new(),
            origin_server_ts: timestamp_ms,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: None,
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
            encryption_shield: None,
        },
        TimelineViewRow::ReadMarker { item_id } => virtual_row_dto("read_marker", item_id),
        TimelineViewRow::UnreadMarker { item_id } => virtual_row_dto("unread_marker", item_id),
        TimelineViewRow::TimelineStart { item_id } => virtual_row_dto("timeline_start", item_id),
        TimelineViewRow::Pagination { item_id, .. } => virtual_row_dto("pagination", item_id),
    }
}

pub(super) fn virtual_row_dto(kind: &str, item_id: String) -> TimelineViewRowDto {
    TimelineViewRowDto {
        kind: kind.to_owned(),
        item_id,
        event_id: String::new(),
        sender: String::new(),
        sender_name: String::new(),
        sender_avatar_url: None,
        body: String::new(),
        origin_server_ts: 0,
        edited: false,
        reply_to_event_id: None,
        reply_preview: None,
        thread_root_event_id: None,
        thread_summary: None,
        poll: None,
        capabilities: None,
        decryption_state: None,
        message_type: None,
        forward_transport: None,
        formatted_body: None,
        agent_card_json: None,
        is_agent_approval: false,
        media_filename: None,
        media_caption: None,
        reactions: Vec::new(),
        media_handle_id: None,
        media_mime_type: None,
        media_width: None,
        media_height: None,
        media_duration_ms: None,
        encryption_shield: None,
    }
}

pub(super) fn timeline_read_state_failed(
    code: &str,
    description: &'static str,
) -> TimelineReadStateError {
    TimelineReadStateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_read_state_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineReadStateError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_read_state_failed(code, TIMELINE_READ_STATE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-event-readback-")
                || code.starts_with("p2-timeline-set-read-state-")
                || code.starts_with("p2-timeline-jump-latest-")
                || code.starts_with("d0.3-timeline-")
                || code.starts_with("v-crypto.6-")
                || code.starts_with("v-timeline-") =>
        {
            timeline_read_state_failed(code, TIMELINE_READ_STATE_OWNER_DESCRIPTION)
        }
        _ => timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn timeline_read_state_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineReadStateError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn read_action_from_str(
    action: &str,
) -> Result<NativeTimelineReadAction, TimelineReadStateError> {
    match action {
        "mark_read" => Ok(NativeTimelineReadAction::MarkRead),
        "mark_unread" => Ok(NativeTimelineReadAction::MarkUnread),
        _ => Err(timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        )),
    }
}

pub(super) fn read_action_as_str(action: NativeTimelineReadAction) -> &'static str {
    match action {
        NativeTimelineReadAction::MarkRead => "mark_read",
        NativeTimelineReadAction::MarkUnread => "mark_unread",
    }
}

pub(super) fn read_intent_from_str(
    intent: &str,
) -> Result<NativeTimelineReadIntent, TimelineReadStateError> {
    match intent {
        "automatic_visibility" => Ok(NativeTimelineReadIntent::AutomaticVisibility),
        "explicit_user" => Ok(NativeTimelineReadIntent::ExplicitUser),
        _ => Err(timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        )),
    }
}

pub(super) fn read_intent_as_str(intent: NativeTimelineReadIntent) -> &'static str {
    match intent {
        NativeTimelineReadIntent::AutomaticVisibility => "automatic_visibility",
        NativeTimelineReadIntent::ExplicitUser => "explicit_user",
    }
}

pub(super) fn timeline_event_item_dto(item: NativeTimelineItem) -> TimelineEventItemDto {
    TimelineEventItemDto {
        item_id: item.item_id,
        event_id: item.event_id,
        sender: item.sender,
        event_type: item.event_type,
        body: item.body,
        origin_server_ts: item.origin_server_ts,
        decryption_state: item.decryption_state.map(|state| match state {
            NativeDecryptionState::Pending => "pending".to_owned(),
            NativeDecryptionState::Unavailable => "unavailable".to_owned(),
        }),
    }
}

impl SharedCore {
    /// Test-only enqueue onto the attach timeline emit queue. Not on UDL.
    #[doc(hidden)]
    pub fn enqueue_timeline_view_update_for_test(
        &self,
        stream_id: String,
        room_id: String,
        revision: u64,
    ) {
        use crate::app::timeline::TimelineReadState;
        let batch = TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            stream_id,
            room_id,
            revision,
            ops: Vec::new(),
            read_state: Some(TimelineReadState {
                visible_tail_event_id: None,
                receipt_tail_event_id: None,
                own_read_event_id: None,
                unread_anchor_event_id: None,
                is_marked_unread: false,
            }),
            pagination: None,
            pinned_event_ids: None,
        };
        if let Ok(mut guard) = self.timeline_view_updates.lock() {
            if guard.len() >= TIMELINE_VIEW_UPDATE_QUEUE_CAP {
                guard.remove(0);
            }
            guard.push(batch);
        }
    }

    pub(super) async fn timeline_read_state_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, TimelineReadStateError> {
        let response = request
            .await
            .map_err(|error| map_timeline_read_state_core_error(no_session, error))?;
        Ok(response)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// Drain queued timeline view-delta summaries. Not `Core.command`.
    ///
    /// NSE forbids this. An empty queue returns an empty list. This is
    /// not Platform::emit. Failed errors stay static.
    pub async fn poll_timeline_view_updates(
        &self,
    ) -> Result<Vec<TimelineViewUpdateDto>, TimelineViewUpdateError> {
        let mut guard = self.timeline_view_updates.lock().map_err(|_| {
            timeline_view_poll_failed(
                TIMELINE_VIEW_POLL_FAILED_CODE,
                TIMELINE_VIEW_POLL_FAILED_DESCRIPTION,
            )
        })?;
        Ok(guard.drain(..).map(timeline_view_update_dto).collect())
    }

    pub async fn timeline_open(
        &self,
        room_id: String,
        position: TimelineOpenPositionDto,
    ) -> Result<TimelineOpenDto, TimelineError> {
        let position = open_position_from_dto(position)?;
        let response = self
            .core
            .timeline_open(crate::core_api::MatrixTimelineOpenRequest { room_id, position })
            .await
            .map_err(map_timeline_open_core_error)?;
        let readback: NativeTimelineOpenReadback = response;
        Ok(TimelineOpenDto {
            schema_version: readback.schema_version,
            stream_id: readback.stream_id,
            position: view_position_dto(readback.position),
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }

    pub async fn timeline_close(&self, stream_id: String) -> Result<bool, TimelineError> {
        let response = self
            .core
            .timeline_close(crate::core_api::MatrixTimelineCloseRequest { stream_id })
            .await
            .map_err(map_timeline_close_core_error)?;
        Ok(response)
    }

    pub async fn timeline_snapshot(
        &self,
        stream_id: String,
    ) -> Result<TimelineSnapshotDto, TimelineError> {
        let response = self
            .core
            .timeline_snapshot(crate::core_api::MatrixTimelineSnapshotRequest { stream_id })
            .await
            .map_err(map_timeline_snapshot_core_error)?;
        let snapshot: TimelineViewSnapshot = response;
        Ok(timeline_snapshot_dto(snapshot))
    }

    pub async fn timeline_retry_decryption(
        &self,
        stream_id: String,
    ) -> Result<bool, TimelineError> {
        let response = self
            .core
            .timeline_retry_decryption(crate::core_api::MatrixTimelineSnapshotRequest { stream_id })
            .await
            .map_err(|error| match error.diagnostic_id.as_deref() {
                Some("p2-timeline-retry-decryption-no-session") => timeline_failed(
                    "p2-timeline-retry-decryption-no-session",
                    TIMELINE_NO_SESSION_DESCRIPTION,
                ),
                Some("v-timeline-view-not-open") => timeline_failed(
                    TIMELINE_VIEW_NOT_OPEN_CODE,
                    TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
                ),
                _ => timeline_failed(
                    "p4-retry-decryption-failed",
                    "Decryption could not be retried.",
                ),
            })?;
        Ok(response)
    }

    pub async fn timeline_paginate(
        &self,
        stream_id: String,
        direction: String,
    ) -> Result<TimelineSnapshotDto, TimelineError> {
        let direction = paginate_direction(&direction)?;
        let response = self
            .core
            .timeline_paginate(crate::core_api::MatrixTimelinePaginateRequest {
                stream_id,
                direction,
            })
            .await
            .map_err(map_timeline_paginate_core_error)?;
        let snapshot: TimelineViewSnapshot = response;
        Ok(timeline_snapshot_dto(snapshot))
    }

    pub async fn timeline_event_readback(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelineEventReadbackDto, TimelineReadStateError> {
        timeline_read_state_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "eventId": event_id,
        }))?;
        let response = self
            .timeline_read_state_command(
                TIMELINE_EVENT_READBACK_NO_SESSION_CODE,
                self.core.timeline_event_readback(
                    crate::core_api::MatrixTimelineEventReadbackRequest { room_id, event_id },
                ),
            )
            .await?;
        let readback: NativeTimelineEventReadback = response;
        Ok(TimelineEventReadbackDto {
            session_generation: readback.session_generation,
            room_id: readback.room_id,
            event_id: readback.event_id,
            item: timeline_event_item_dto(readback.item),
        })
    }

    pub async fn timeline_set_read_state(
        &self,
        stream_id: String,
        action: String,
        intent: String,
        observed_live_tail_event_id: Option<String>,
    ) -> Result<TimelineReadStateDto, TimelineReadStateError> {
        let action = read_action_from_str(&action)?;
        let intent = read_intent_from_str(&intent)?;
        timeline_read_state_envelope_payload(serde_json::json!({
            "streamId": stream_id,
            "action": read_action_as_str(action),
            "intent": read_intent_as_str(intent),
            "observedLiveTailEventId": observed_live_tail_event_id,
        }))?;
        let response = self
            .timeline_read_state_command(
                TIMELINE_SET_READ_STATE_NO_SESSION_CODE,
                self.core.timeline_set_read_state(
                    crate::core_api::MatrixTimelineSetReadStateRequest {
                        stream_id,
                        action,
                        intent,
                        observed_live_tail_event_id,
                    },
                ),
            )
            .await?;
        let readback: NativeTimelineReadStateReadback = response;
        Ok(TimelineReadStateDto {
            action: read_action_as_str(readback.action).to_owned(),
            receipt_sent: readback.receipt_sent,
            acknowledged_event_id: readback.acknowledged_event_id,
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }

    pub async fn timeline_jump_latest(
        &self,
        stream_id: String,
    ) -> Result<TimelineOpenDto, TimelineReadStateError> {
        timeline_read_state_envelope_payload(serde_json::json!({
            "streamId": stream_id,
        }))?;
        let response = self
            .timeline_read_state_command(
                TIMELINE_JUMP_LATEST_NO_SESSION_CODE,
                self.core
                    .timeline_jump_latest(crate::core_api::MatrixTimelineJumpLatestRequest {
                        stream_id,
                    }),
            )
            .await?;
        let readback: NativeTimelineOpenReadback = response;
        Ok(TimelineOpenDto {
            schema_version: readback.schema_version,
            stream_id: readback.stream_id,
            position: view_position_dto(readback.position),
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }
}
