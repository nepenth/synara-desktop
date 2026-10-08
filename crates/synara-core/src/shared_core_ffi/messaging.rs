//! Typed SharedCore operations and projections for messaging.

use super::*;

/// Privacy-safe reaction count on a view row. Senders are user ids and
/// optional annotation ids only; no tokens or ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineViewReactionDto {
    pub key: String,
    pub count: u32,
    pub own: Option<bool>,
    pub senders: Vec<TimelineReactionSenderDto>,
}

/// Privacy-safe reaction sender. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineReactionSenderDto {
    pub user_id: String,
    pub reaction_event_id: Option<String>,
}

/// Privacy-safe aggregated reaction. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineReactionDto {
    pub key: String,
    pub count: u32,
    pub me: bool,
    pub senders: Vec<TimelineReactionSenderDto>,
}

/// Privacy-safe reaction mutation ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineReactionMutationDto {
    pub room_id: String,
    pub target_event_id: String,
    pub key: String,
    pub mutation: String,
    pub readback: Option<TimelineReactionDto>,
}

/// Privacy-safe result of the shared-core approval decision route.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AgentApprovalDecisionDto {
    pub room_id: String,
    pub event_id: String,
    pub status: String,
    pub reaction: Option<TimelineReactionMutationDto>,
}

/// Static fail-closed timeline reaction error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineReactionError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineReactionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineReactionError {}

/// Privacy-safe composer reply-draft preview. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ComposerReplyDraftPreviewDto {
    pub event_id: String,
    pub sender_id: String,
    pub body: String,
    pub formatted_body: Option<String>,
    pub thread_root_event_id: Option<String>,
}

/// Privacy-safe composer reply-draft readback from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ComposerReplyDraftDto {
    pub schema_version: u32,
    pub room_id: String,
    pub status: String,
    pub draft: Option<ComposerReplyDraftPreviewDto>,
}

/// Static fail-closed composer reply-draft error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum ComposerReplyDraftError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ComposerReplyDraftError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ComposerReplyDraftError {}

/// Privacy-safe send-text write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SendTextDto {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: String,
}

/// Privacy-safe agent-approval write acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AgentApprovalSendDto {
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed agent-approval error. Input values are never echoed.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum AgentApprovalSendError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for AgentApprovalSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for AgentApprovalSendError {}

/// Static fail-closed send-text error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum SendTextError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SendTextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SendTextError {}

/// Privacy-safe send-poll write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SendPollDto {
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed send-poll error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum SendPollError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SendPollError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SendPollError {}

/// Privacy-safe edit-message write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditMessageDto {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: String,
}

/// Static fail-closed edit-message error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum EditMessageError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for EditMessageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for EditMessageError {}

/// Privacy-safe poll-respond write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PollRespondDto {
    pub room_id: String,
    pub poll_event_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed poll-respond error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum PollRespondError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PollRespondError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PollRespondError {}

/// Privacy-safe timeline edit/redact/report write ack from the registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineMutateDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline edit/redact/report error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineMutateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineMutateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineMutateError {}

/// Privacy-safe timeline pin/unpin write ack from the registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelinePinDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline pin/unpin error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelinePinError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelinePinError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelinePinError {}

/// Privacy-safe timeline poll-vote / call-decline write ack from the
/// registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineVoteDeclineDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline poll-vote / call-decline error. Fields are
/// source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineVoteDeclineError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineVoteDeclineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineVoteDeclineError {}

/// Privacy-safe timeline forward write ack from the registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineForwardDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline forward error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineForwardError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineForwardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineForwardError {}

pub(super) fn view_reaction_dtos(reactions: Vec<TimelineReaction>) -> Vec<TimelineViewReactionDto> {
    reactions
        .into_iter()
        .map(|reaction| TimelineViewReactionDto {
            key: reaction.key,
            count: reaction.count,
            own: reaction.own,
            senders: reaction
                .senders
                .into_iter()
                .map(|sender| TimelineReactionSenderDto {
                    user_id: sender.user_id,
                    reaction_event_id: sender.reaction_event_id,
                })
                .collect(),
        })
        .collect()
}

pub(super) fn timeline_reaction_failed(
    code: &str,
    description: &'static str,
) -> TimelineReactionError {
    TimelineReactionError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_reaction_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineReactionError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_reaction_failed(code, TIMELINE_REACTION_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-reaction-ensure-")
                || code.starts_with("p2-reaction-redact-")
                || code.starts_with("p2-timeline-reaction-toggle-")
                || code.starts_with("d0.3-timeline-")
                || code.starts_with("v-crypto.6-")
                || code.starts_with("v-send.2-reaction-")
                || code.starts_with("agent-approval-") =>
        {
            timeline_reaction_failed(code, TIMELINE_REACTION_OWNER_DESCRIPTION)
        }
        _ => timeline_reaction_failed(
            TIMELINE_REACTION_FAILED_CODE,
            TIMELINE_REACTION_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn timeline_reaction_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineReactionError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_reaction_failed(
            TIMELINE_REACTION_FAILED_CODE,
            TIMELINE_REACTION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn reaction_mutation_as_str(mutation: NativeReactionMutation) -> &'static str {
    match mutation {
        NativeReactionMutation::Added => "added",
        NativeReactionMutation::Removed => "removed",
        NativeReactionMutation::AlreadyPresent => "already_present",
        NativeReactionMutation::Redacted => "redacted",
    }
}

pub(super) fn timeline_reaction_sender_dto(
    sender: NativeTimelineReactionSender,
) -> TimelineReactionSenderDto {
    TimelineReactionSenderDto {
        user_id: sender.user_id,
        reaction_event_id: sender.reaction_event_id,
    }
}

pub(super) fn timeline_reaction_dto(reaction: NativeTimelineReaction) -> TimelineReactionDto {
    TimelineReactionDto {
        key: reaction.key,
        count: reaction.count,
        me: reaction.me,
        senders: reaction
            .senders
            .into_iter()
            .map(timeline_reaction_sender_dto)
            .collect(),
    }
}

pub(super) fn timeline_reaction_mutation_dto(
    result: NativeReactionMutationResult,
) -> TimelineReactionMutationDto {
    TimelineReactionMutationDto {
        room_id: result.room_id,
        target_event_id: result.target_event_id,
        key: result.key,
        mutation: reaction_mutation_as_str(result.mutation).to_owned(),
        readback: result.readback.map(timeline_reaction_dto),
    }
}

pub(super) fn composer_reply_draft_failed(
    code: &str,
    description: &'static str,
) -> ComposerReplyDraftError {
    ComposerReplyDraftError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_composer_reply_draft_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ComposerReplyDraftError {
    match error.diagnostic_id.as_deref() {
        Some(code)
            if code == COMPOSER_SET_REPLY_DRAFT_NO_SESSION_CODE
                || code == COMPOSER_GET_REPLY_DRAFT_NO_SESSION_CODE
                || code == COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE =>
        {
            // The revision-less clear compatibility route snapshots through
            // the get command before it performs the atomic clear. Preserve
            // the public operation's diagnostic rather than leaking that
            // internal owner hop to callers.
            composer_reply_draft_failed(no_session, COMPOSER_REPLY_DRAFT_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-composer-set-reply-draft-")
                || code.starts_with("p2-composer-get-reply-draft-")
                || code.starts_with("p2-composer-clear-reply-draft-")
                || code.starts_with("v-timeline-reply-draft-")
                || code == "d0.4-send-invalid-room-id" =>
        {
            composer_reply_draft_failed(code, COMPOSER_REPLY_DRAFT_OWNER_DESCRIPTION)
        }
        _ => composer_reply_draft_failed(
            COMPOSER_REPLY_DRAFT_FAILED_CODE,
            COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn composer_reply_draft_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, ComposerReplyDraftError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(composer_reply_draft_failed(
            COMPOSER_REPLY_DRAFT_FAILED_CODE,
            COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ComposerReplyDraftReadbackWire {
    pub(super) schema_version: u32,
    pub(super) room_id: String,
    pub(super) status: String,
    #[serde(default)]
    pub(super) draft: Option<NativeComposerReplyDraft>,
}

pub(super) fn composer_reply_draft_preview_dto(
    draft: NativeComposerReplyDraft,
) -> ComposerReplyDraftPreviewDto {
    ComposerReplyDraftPreviewDto {
        event_id: draft.event_id,
        sender_id: draft.sender_id,
        body: draft.body,
        formatted_body: draft.formatted_body,
        thread_root_event_id: draft.thread_root_event_id,
    }
}

pub(super) fn composer_reply_draft_dto(
    readback: ComposerReplyDraftReadbackWire,
) -> ComposerReplyDraftDto {
    ComposerReplyDraftDto {
        schema_version: readback.schema_version,
        room_id: readback.room_id,
        status: readback.status,
        draft: readback.draft.map(composer_reply_draft_preview_dto),
    }
}

pub(super) fn send_text_failed(code: &str, description: &'static str) -> SendTextError {
    SendTextError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn agent_approval_failed(
    code: &str,
    description: &'static str,
) -> AgentApprovalSendError {
    AgentApprovalSendError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_send_text_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> SendTextError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            send_text_failed(code, SEND_TEXT_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-send-text-")
                || code.starts_with("d0.4-send-")
                || code.starts_with("v-send.4-")
                || code.starts_with("v-send.5-")
                || code.starts_with("p6.1-") =>
        {
            send_text_failed(code, SEND_TEXT_OWNER_DESCRIPTION)
        }
        _ => send_text_failed(SEND_TEXT_FAILED_CODE, SEND_TEXT_FAILED_DESCRIPTION),
    }
}

pub(super) fn send_text_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SendTextError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(send_text_failed(
            SEND_TEXT_FAILED_CODE,
            SEND_TEXT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SendTextResultWire {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) local_txn_id: String,
    pub(super) status: String,
}

pub(super) fn send_text_dto(result: SendTextResultWire) -> SendTextDto {
    SendTextDto {
        room_id: result.room_id,
        event_id: result.event_id,
        local_txn_id: result.local_txn_id,
        status: result.status,
    }
}

pub(super) fn send_poll_failed(code: &str, description: &'static str) -> SendPollError {
    SendPollError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_send_poll_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> SendPollError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            send_poll_failed(code, SEND_POLL_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-send-poll-")
                || code.starts_with("v-send.3-poll-")
                || code.starts_with("d0.4-send-")
                || code.starts_with("v-send.5-") =>
        {
            send_poll_failed(code, SEND_POLL_OWNER_DESCRIPTION)
        }
        _ => send_poll_failed(SEND_POLL_FAILED_CODE, SEND_POLL_FAILED_DESCRIPTION),
    }
}

pub(super) fn send_poll_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SendPollError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(send_poll_failed(
            SEND_POLL_FAILED_CODE,
            SEND_POLL_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SendPollResultWire {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) status: String,
}

pub(super) fn send_poll_dto(result: SendPollResultWire) -> SendPollDto {
    SendPollDto {
        room_id: result.room_id,
        event_id: result.event_id,
        status: result.status,
    }
}

pub(super) fn edit_message_failed(code: &str, description: &'static str) -> EditMessageError {
    EditMessageError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_edit_message_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> EditMessageError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            edit_message_failed(code, EDIT_MESSAGE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-edit-message-")
                || code.starts_with("v-send.r-edit-")
                || code.starts_with("d0.4-send-")
                || code.starts_with("v-send.4-")
                || code.starts_with("p6.1-") =>
        {
            edit_message_failed(code, EDIT_MESSAGE_OWNER_DESCRIPTION)
        }
        _ => edit_message_failed(EDIT_MESSAGE_FAILED_CODE, EDIT_MESSAGE_FAILED_DESCRIPTION),
    }
}

pub(super) fn edit_message_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, EditMessageError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(edit_message_failed(
            EDIT_MESSAGE_FAILED_CODE,
            EDIT_MESSAGE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EditMessageResultWire {
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) local_txn_id: String,
    pub(super) status: String,
}

pub(super) fn edit_message_dto(result: EditMessageResultWire) -> EditMessageDto {
    EditMessageDto {
        room_id: result.room_id,
        event_id: result.event_id,
        local_txn_id: result.local_txn_id,
        status: result.status,
    }
}

pub(super) fn poll_respond_failed(code: &str, description: &'static str) -> PollRespondError {
    PollRespondError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_poll_respond_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PollRespondError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            poll_respond_failed(code, POLL_RESPOND_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-poll-respond-")
                || code.starts_with("v-send.3-poll-")
                || code.starts_with("d0.4-send-") =>
        {
            poll_respond_failed(code, POLL_RESPOND_OWNER_DESCRIPTION)
        }
        _ => poll_respond_failed(POLL_RESPOND_FAILED_CODE, POLL_RESPOND_FAILED_DESCRIPTION),
    }
}

pub(super) fn poll_respond_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, PollRespondError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(poll_respond_failed(
            POLL_RESPOND_FAILED_CODE,
            POLL_RESPOND_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PollRespondResultWire {
    pub(super) room_id: String,
    pub(super) poll_event_id: String,
    pub(super) event_id: String,
    pub(super) status: String,
}

pub(super) fn poll_respond_dto(result: PollRespondResultWire) -> PollRespondDto {
    PollRespondDto {
        room_id: result.room_id,
        poll_event_id: result.poll_event_id,
        event_id: result.event_id,
        status: result.status,
    }
}

pub(super) fn timeline_mutate_failed(code: &str, description: &'static str) -> TimelineMutateError {
    TimelineMutateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_mutate_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineMutateError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_mutate_failed(code, TIMELINE_MUTATE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-edit-text-")
                || code.starts_with("p2-timeline-redact-")
                || code.starts_with("p2-timeline-report-")
                || code.starts_with("v-timeline-edit-")
                || code.starts_with("v-timeline-redact-")
                || code.starts_with("v-timeline-report-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_mutate_failed(code, TIMELINE_MUTATE_OWNER_DESCRIPTION)
        }
        _ => timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn timeline_mutate_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineMutateError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TimelineMutateResultWire {
    pub(super) schema_version: u32,
    pub(super) action: String,
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) status: String,
}

pub(super) fn closed_timeline_mutate_action(value: &str) -> Option<&'static str> {
    match value {
        "edit_text" => Some("edit_text"),
        "redact" => Some("redact"),
        "report" => Some("report"),
        _ => None,
    }
}

pub(super) fn closed_timeline_mutate_status(value: &str) -> Option<&'static str> {
    match value {
        "sent" => Some("sent"),
        // An edit the SDK still holds; it keeps retrying in order.
        "queued" => Some("queued"),
        "redacted" => Some("redacted"),
        "reported" => Some("reported"),
        _ => None,
    }
}

pub(super) fn timeline_mutate_dto(
    result: TimelineMutateResultWire,
) -> Result<TimelineMutateDto, TimelineMutateError> {
    let action = closed_timeline_mutate_action(&result.action).ok_or_else(|| {
        timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_timeline_mutate_status(&result.status).ok_or_else(|| {
        timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        )
    })?;
    Ok(TimelineMutateDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

pub(super) fn timeline_pin_failed(code: &str, description: &'static str) -> TimelinePinError {
    TimelinePinError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_pin_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelinePinError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_pin_failed(code, TIMELINE_PIN_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-pin-")
                || code.starts_with("p2-timeline-unpin-")
                || code.starts_with("v-timeline-pin-")
                || code.starts_with("v-timeline-unpin-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_pin_failed(code, TIMELINE_PIN_OWNER_DESCRIPTION)
        }
        _ => timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION),
    }
}

pub(super) fn timeline_pin_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelinePinError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_pin_failed(
            TIMELINE_PIN_FAILED_CODE,
            TIMELINE_PIN_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TimelinePinResultWire {
    pub(super) schema_version: u32,
    pub(super) action: String,
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) status: String,
}

pub(super) fn closed_timeline_pin_action(value: &str) -> Option<&'static str> {
    match value {
        "pin" => Some("pin"),
        "unpin" => Some("unpin"),
        _ => None,
    }
}

pub(super) fn closed_timeline_pin_status(value: &str) -> Option<&'static str> {
    match value {
        "pinned" => Some("pinned"),
        "unpinned" => Some("unpinned"),
        "already_pinned" => Some("already_pinned"),
        "already_unpinned" => Some("already_unpinned"),
        _ => None,
    }
}

pub(super) fn timeline_pin_dto(
    result: TimelinePinResultWire,
) -> Result<TimelinePinDto, TimelinePinError> {
    let action = closed_timeline_pin_action(&result.action).ok_or_else(|| {
        timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION)
    })?;
    let status = closed_timeline_pin_status(&result.status).ok_or_else(|| {
        timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION)
    })?;
    Ok(TimelinePinDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

pub(super) fn timeline_vote_decline_failed(
    code: &str,
    description: &'static str,
) -> TimelineVoteDeclineError {
    TimelineVoteDeclineError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_vote_decline_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineVoteDeclineError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_vote_decline_failed(code, TIMELINE_VOTE_DECLINE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-poll-vote-")
                || code.starts_with("p2-timeline-call-decline-")
                || code.starts_with("v-timeline-poll-vote-")
                || code.starts_with("v-timeline-call-decline-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_vote_decline_failed(code, TIMELINE_VOTE_DECLINE_OWNER_DESCRIPTION)
        }
        _ => timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn timeline_vote_decline_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineVoteDeclineError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TimelineVoteDeclineResultWire {
    pub(super) schema_version: u32,
    pub(super) action: String,
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) status: String,
}

pub(super) fn closed_timeline_vote_decline_action(value: &str) -> Option<&'static str> {
    match value {
        "poll_vote" => Some("poll_vote"),
        "call_decline" => Some("call_decline"),
        _ => None,
    }
}

pub(super) fn closed_timeline_vote_decline_status(value: &str) -> Option<&'static str> {
    match value {
        "voted" => Some("voted"),
        "queued" => Some("queued"),
        "declined" => Some("declined"),
        _ => None,
    }
}

pub(super) fn timeline_vote_decline_dto(
    result: TimelineVoteDeclineResultWire,
) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
    let action = closed_timeline_vote_decline_action(&result.action).ok_or_else(|| {
        timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_timeline_vote_decline_status(&result.status).ok_or_else(|| {
        timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        )
    })?;
    Ok(TimelineVoteDeclineDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

pub(super) fn timeline_forward_failed(
    code: &str,
    description: &'static str,
) -> TimelineForwardError {
    TimelineForwardError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_timeline_forward_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineForwardError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_forward_failed(code, TIMELINE_FORWARD_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-forward-text-")
                || code.starts_with("p2-timeline-forward-media-")
                || code.starts_with("v-timeline-forward-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_forward_failed(code, TIMELINE_FORWARD_OWNER_DESCRIPTION)
        }
        _ => timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn timeline_forward_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineForwardError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TimelineForwardResultWire {
    pub(super) schema_version: u32,
    pub(super) action: String,
    pub(super) room_id: String,
    pub(super) event_id: String,
    pub(super) status: String,
}

pub(super) fn closed_timeline_forward_action(value: &str) -> Option<&'static str> {
    match value {
        "forward_text" => Some("forward_text"),
        "forward_media" => Some("forward_media"),
        _ => None,
    }
}

pub(super) fn closed_timeline_forward_status(value: &str) -> Option<&'static str> {
    match value {
        "sent" => Some("sent"),
        "queued" => Some("queued"),
        _ => None,
    }
}

pub(super) fn timeline_forward_dto(
    result: TimelineForwardResultWire,
) -> Result<TimelineForwardDto, TimelineForwardError> {
    let action = closed_timeline_forward_action(&result.action).ok_or_else(|| {
        timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_timeline_forward_status(&result.status).ok_or_else(|| {
        timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        )
    })?;
    Ok(TimelineForwardDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

impl SharedCore {
    pub(super) async fn timeline_reaction_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        let payload = timeline_reaction_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_REACTION_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_reaction_core_error(no_session, error))?;
        let result: NativeReactionMutationResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_reaction_failed(
                    TIMELINE_REACTION_FAILED_CODE,
                    TIMELINE_REACTION_FAILED_DESCRIPTION,
                )
            })?;
        Ok(timeline_reaction_mutation_dto(result))
    }

    pub(super) async fn composer_reply_draft_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        let readback = self
            .composer_reply_draft_command_wire(command, no_session, payload)
            .await?;
        Ok(composer_reply_draft_dto(readback))
    }

    pub(super) async fn composer_reply_draft_command_wire(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<ComposerReplyDraftReadbackWire, ComposerReplyDraftError> {
        let payload = composer_reply_draft_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: COMPOSER_REPLY_DRAFT_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_composer_reply_draft_core_error(no_session, error))?;
        serde_json::from_value(response.payload).map_err(|_| {
            composer_reply_draft_failed(
                COMPOSER_REPLY_DRAFT_FAILED_CODE,
                COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION,
            )
        })
    }

    pub(super) async fn send_text_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<SendTextDto, SendTextError> {
        let payload = send_text_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SEND_TEXT_COMMAND.to_owned(),
                session_generation: SEND_TEXT_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_send_text_core_error(SEND_TEXT_NO_SESSION_CODE, error))?;
        let result: SendTextResultWire = serde_json::from_value(response.payload)
            .map_err(|_| send_text_failed(SEND_TEXT_FAILED_CODE, SEND_TEXT_FAILED_DESCRIPTION))?;
        Ok(send_text_dto(result))
    }

    pub(super) async fn send_poll_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<SendPollDto, SendPollError> {
        let payload = send_poll_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SEND_POLL_COMMAND.to_owned(),
                session_generation: SEND_POLL_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_send_poll_core_error(SEND_POLL_NO_SESSION_CODE, error))?;
        let result: SendPollResultWire = serde_json::from_value(response.payload)
            .map_err(|_| send_poll_failed(SEND_POLL_FAILED_CODE, SEND_POLL_FAILED_DESCRIPTION))?;
        Ok(send_poll_dto(result))
    }

    pub(super) async fn edit_message_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<EditMessageDto, EditMessageError> {
        let payload = edit_message_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: EDIT_MESSAGE_COMMAND.to_owned(),
                session_generation: EDIT_MESSAGE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_edit_message_core_error(EDIT_MESSAGE_NO_SESSION_CODE, error))?;
        let result: EditMessageResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                edit_message_failed(EDIT_MESSAGE_FAILED_CODE, EDIT_MESSAGE_FAILED_DESCRIPTION)
            })?;
        Ok(edit_message_dto(result))
    }

    pub(super) async fn poll_respond_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<PollRespondDto, PollRespondError> {
        let payload = poll_respond_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: POLL_RESPOND_COMMAND.to_owned(),
                session_generation: POLL_RESPOND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_poll_respond_core_error(POLL_RESPOND_NO_SESSION_CODE, error))?;
        let result: PollRespondResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                poll_respond_failed(POLL_RESPOND_FAILED_CODE, POLL_RESPOND_FAILED_DESCRIPTION)
            })?;
        Ok(poll_respond_dto(result))
    }

    pub(super) async fn timeline_mutate_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        let payload = timeline_mutate_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_MUTATE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_mutate_core_error(no_session, error))?;
        let result: TimelineMutateResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_mutate_failed(
                    TIMELINE_MUTATE_FAILED_CODE,
                    TIMELINE_MUTATE_FAILED_DESCRIPTION,
                )
            })?;
        timeline_mutate_dto(result)
    }

    pub(super) async fn timeline_pin_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelinePinDto, TimelinePinError> {
        let payload = timeline_pin_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_PIN_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_pin_core_error(no_session, error))?;
        let result: TimelinePinResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION)
            })?;
        timeline_pin_dto(result)
    }

    pub(super) async fn timeline_vote_decline_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
        let payload = timeline_vote_decline_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_VOTE_DECLINE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_vote_decline_core_error(no_session, error))?;
        let result: TimelineVoteDeclineResultWire = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_vote_decline_failed(
                    TIMELINE_VOTE_DECLINE_FAILED_CODE,
                    TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
                )
            })?;
        timeline_vote_decline_dto(result)
    }

    pub(super) async fn timeline_forward_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineForwardDto, TimelineForwardError> {
        let payload = timeline_forward_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_FORWARD_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_forward_core_error(no_session, error))?;
        let result: TimelineForwardResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_forward_failed(
                    TIMELINE_FORWARD_FAILED_CODE,
                    TIMELINE_FORWARD_FAILED_DESCRIPTION,
                )
            })?;
        timeline_forward_dto(result)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn reaction_ensure(
        &self,
        room_id: String,
        event_id: String,
        key: String,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        self.timeline_reaction_command(
            REACTION_ENSURE_COMMAND,
            REACTION_ENSURE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "key": key,
            }),
        )
        .await
    }

    pub async fn agent_approval_decide(
        &self,
        room_id: String,
        event_id: String,
        action_id: String,
    ) -> Result<AgentApprovalDecisionDto, TimelineReactionError> {
        let payload = timeline_reaction_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "eventId": event_id,
            "actionId": action_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: AGENT_APPROVAL_DECIDE_COMMAND.to_owned(),
                session_generation: TIMELINE_REACTION_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_timeline_reaction_core_error("agent-approval-no-session", error)
            })?;
        let result: NativeAgentApprovalDecisionResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_reaction_failed(
                    TIMELINE_REACTION_FAILED_CODE,
                    TIMELINE_REACTION_FAILED_DESCRIPTION,
                )
            })?;
        Ok(AgentApprovalDecisionDto {
            room_id: result.room_id,
            event_id: result.event_id,
            status: match result.status {
                crate::app::agent_approvals::AgentApprovalDecisionStatus::Applied => "applied",
                crate::app::agent_approvals::AgentApprovalDecisionStatus::AlreadyDecided => {
                    "already_decided"
                }
            }
            .to_owned(),
            reaction: result.reaction.map(timeline_reaction_mutation_dto),
        })
    }

    pub async fn reaction_redact(
        &self,
        room_id: String,
        target_event_id: String,
        reaction_event_id: String,
        key: String,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        self.timeline_reaction_command(
            REACTION_REDACT_COMMAND,
            REACTION_REDACT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "targetEventId": target_event_id,
                "reactionEventId": reaction_event_id,
                "key": key,
            }),
        )
        .await
    }

    pub async fn timeline_reaction_toggle(
        &self,
        room_id: String,
        event_id: String,
        key: String,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        self.timeline_reaction_command(
            TIMELINE_REACTION_TOGGLE_COMMAND,
            TIMELINE_REACTION_TOGGLE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "key": key,
            }),
        )
        .await
    }

    pub async fn composer_set_reply_draft(
        &self,
        room_id: String,
        event_id: String,
        start_thread: bool,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        self.composer_reply_draft_command(
            COMPOSER_SET_REPLY_DRAFT_COMMAND,
            COMPOSER_SET_REPLY_DRAFT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "startThread": start_thread,
            }),
        )
        .await
    }

    pub async fn composer_get_reply_draft(
        &self,
        room_id: String,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        self.composer_reply_draft_command(
            COMPOSER_GET_REPLY_DRAFT_COMMAND,
            COMPOSER_GET_REPLY_DRAFT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
            }),
        )
        .await
    }

    pub async fn composer_clear_reply_draft(
        &self,
        room_id: String,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        // This compatibility wrapper predates Core-issued draft revisions.
        // Snapshot the current revision, then let the Core owner perform the
        // atomic comparison. A selection made between these commands is
        // intentionally preserved rather than cleared by the older caller.
        let current = self
            .composer_reply_draft_command_wire(
                COMPOSER_GET_REPLY_DRAFT_COMMAND,
                COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE,
                serde_json::json!({
                    "roomId": room_id,
                }),
            )
            .await?;
        let expected_draft_revision = current
            .draft
            .as_ref()
            .map_or(0, |draft| draft.draft_revision);
        self.composer_reply_draft_command(
            COMPOSER_CLEAR_REPLY_DRAFT_COMMAND,
            COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "expectedDraftRevision": expected_draft_revision,
            }),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_text(
        &self,
        room_id: String,
        body: String,
        msg_type: Option<String>,
        formatted_body: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
        reply_to: Option<String>,
        thread_root: Option<String>,
        txn_id: Option<String>,
    ) -> Result<SendTextDto, SendTextError> {
        self.send_text_command(serde_json::json!({
            "roomId": room_id,
            "body": body,
            "msgType": msg_type,
            "formattedBody": formatted_body,
            "mentionUserIds": mention_user_ids,
            "mentionRoom": mention_room,
            "replyTo": reply_to,
            "threadRoot": thread_root,
            "txnId": txn_id,
        }))
        .await
    }

    pub async fn send_poll(
        &self,
        room_id: String,
        question: String,
        answers: Vec<String>,
        max_selections: u32,
        thread_root: Option<String>,
        reply_to: Option<String>,
    ) -> Result<SendPollDto, SendPollError> {
        self.send_poll_command(serde_json::json!({
            "roomId": room_id,
            "question": question,
            "answers": answers,
            "maxSelections": max_selections,
            "threadRoot": thread_root,
            "replyTo": reply_to,
        }))
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn edit_message(
        &self,
        room_id: String,
        event_id: String,
        body: String,
        msg_type: Option<String>,
        formatted_body: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
        txn_id: Option<String>,
    ) -> Result<EditMessageDto, EditMessageError> {
        self.edit_message_command(serde_json::json!({
            "roomId": room_id,
            "eventId": event_id,
            "body": body,
            "msgType": msg_type,
            "formattedBody": formatted_body,
            "mentionUserIds": mention_user_ids,
            "mentionRoom": mention_room,
            "txnId": txn_id,
        }))
        .await
    }

    pub async fn poll_respond(
        &self,
        room_id: String,
        poll_event_id: String,
        answer_ids: Vec<String>,
    ) -> Result<PollRespondDto, PollRespondError> {
        self.poll_respond_command(serde_json::json!({
            "roomId": room_id,
            "pollEventId": poll_event_id,
            "answerIds": answer_ids,
        }))
        .await
    }

    pub async fn timeline_edit_text(
        &self,
        room_id: String,
        event_id: String,
        body: String,
        formatted_body: Option<String>,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        self.timeline_mutate_command(
            TIMELINE_EDIT_TEXT_COMMAND,
            TIMELINE_EDIT_TEXT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "body": body,
                "formattedBody": formatted_body,
            }),
        )
        .await
    }

    pub async fn timeline_redact(
        &self,
        room_id: String,
        event_id: String,
        reason: Option<String>,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        self.timeline_mutate_command(
            TIMELINE_REDACT_COMMAND,
            TIMELINE_REDACT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "reason": reason,
            }),
        )
        .await
    }

    pub async fn timeline_report(
        &self,
        room_id: String,
        event_id: String,
        reason: Option<String>,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        self.timeline_mutate_command(
            TIMELINE_REPORT_COMMAND,
            TIMELINE_REPORT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "reason": reason,
            }),
        )
        .await
    }

    pub async fn timeline_pin(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelinePinDto, TimelinePinError> {
        self.timeline_pin_command(
            TIMELINE_PIN_COMMAND,
            TIMELINE_PIN_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        )
        .await
    }

    pub async fn timeline_unpin(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelinePinDto, TimelinePinError> {
        self.timeline_pin_command(
            TIMELINE_UNPIN_COMMAND,
            TIMELINE_UNPIN_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        )
        .await
    }

    pub async fn timeline_poll_vote(
        &self,
        room_id: String,
        event_id: String,
        answer_ids: Vec<String>,
    ) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
        self.timeline_vote_decline_command(
            TIMELINE_POLL_VOTE_COMMAND,
            TIMELINE_POLL_VOTE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "answerIds": answer_ids,
            }),
        )
        .await
    }

    pub async fn timeline_call_decline(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
        self.timeline_vote_decline_command(
            TIMELINE_CALL_DECLINE_COMMAND,
            TIMELINE_CALL_DECLINE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        )
        .await
    }

    pub async fn timeline_forward_text(
        &self,
        source_room_id: String,
        event_id: String,
        target_room_id: String,
        as_quote: bool,
        confirmed_encryption_downgrade: bool,
    ) -> Result<TimelineForwardDto, TimelineForwardError> {
        self.timeline_forward_command(
            TIMELINE_FORWARD_TEXT_COMMAND,
            TIMELINE_FORWARD_TEXT_NO_SESSION_CODE,
            serde_json::json!({
                "sourceRoomId": source_room_id,
                "eventId": event_id,
                "targetRoomId": target_room_id,
                "asQuote": as_quote,
                "confirmedEncryptionDowngrade": confirmed_encryption_downgrade,
            }),
        )
        .await
    }

    pub async fn send_agent_approval(
        &self,
        room_id: String,
        action_id: String,
        action_title: String,
        decision: String,
        source_event_id: Option<String>,
        created_at: u64,
    ) -> Result<AgentApprovalSendDto, AgentApprovalSendError> {
        let size = room_id.len()
            + action_id.len()
            + action_title.len()
            + decision.len()
            + source_event_id.as_deref().map(str::len).unwrap_or_default();
        if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
            || action_id.trim().is_empty()
            || action_title.trim().is_empty()
            || !matches!(decision.as_str(), "approve" | "reject")
        {
            return Err(agent_approval_failed(
                AGENT_APPROVAL_INVALID_CODE,
                AGENT_APPROVAL_INVALID_DESCRIPTION,
            ));
        }
        let room_id = matrix_sdk::ruma::OwnedRoomId::try_from(room_id.trim()).map_err(|_| {
            agent_approval_failed(
                AGENT_APPROVAL_INVALID_CODE,
                AGENT_APPROVAL_INVALID_DESCRIPTION,
            )
        })?;
        if let Some(event_id) = source_event_id.as_deref() {
            matrix_sdk::ruma::OwnedEventId::try_from(event_id.trim()).map_err(|_| {
                agent_approval_failed(
                    AGENT_APPROVAL_INVALID_CODE,
                    AGENT_APPROVAL_INVALID_DESCRIPTION,
                )
            })?;
        }
        let client = self.retained_client().map_err(|_| {
            agent_approval_failed(
                AGENT_APPROVAL_NO_SESSION_CODE,
                AGENT_APPROVAL_NO_SESSION_DESCRIPTION,
            )
        })?;
        let room = client.get_room(&room_id).ok_or_else(|| {
            agent_approval_failed(
                AGENT_APPROVAL_FAILED_CODE,
                AGENT_APPROVAL_FAILED_DESCRIPTION,
            )
        })?;
        let content = serde_json::json!({
            "msgtype": "m.notice",
            "body": format!("{} agent action: {}", if decision == "approve" { "Approved" } else { "Rejected" }, action_title),
            "in.synara.agent.action": {
                "version": 1,
                "action_id": action_id,
                "action_title": action_title,
                "decision": decision,
                "source_event_id": source_event_id,
                "created_at": created_at,
            }
        });
        let response = room
            .send_raw("m.room.message", content)
            .await
            .map_err(|_| {
                agent_approval_failed(
                    AGENT_APPROVAL_FAILED_CODE,
                    AGENT_APPROVAL_FAILED_DESCRIPTION,
                )
            })?;
        Ok(AgentApprovalSendDto {
            event_id: response.response.event_id.to_string(),
            status: "sent".to_owned(),
        })
    }
}
