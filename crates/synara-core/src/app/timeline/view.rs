//! SDK-neutral V-TIMELINE view contract.
//!
//! These DTOs are the target presenter boundary, not a serialization of SDK
//! events. In particular, they exclude Matrix client/room/event objects, raw
//! content, ciphertext, MXC URIs, and media bytes. Projection and the delta
//! subscription are deliberately separate follow-up work; this module fixes
//! the stable shape that those owners must produce.

use std::collections::HashMap;
use std::sync::Arc;

use eyeball_im::VectorDiff;
use matrix_sdk::ruma::{
    events::{
        room::message::{MessageFormat, MessageType},
        StateEventContentChange,
    },
    UserId as RumaUserId,
};
use matrix_sdk_ui::timeline::{
    AnyOtherStateEventContentChange, EventTimelineItem, MemberProfileChange, MembershipChange,
    MsgLikeKind, OtherState, RoomMembershipChange, TimelineDetails, TimelineEventItemId,
    TimelineItem as SdkTimelineItem, TimelineItemContent, VirtualTimelineItem,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::app::agent_approvals::is_eligible_agent_approval_prompt;
use crate::dto::{EventId, RoomId, TimelineItemId, UserId};

use super::TimelineMediaRegistry;

pub const TIMELINE_VIEW_SCHEMA_VERSION: u32 = 1;
pub const NATIVE_TIMELINE_VIEW_UPDATED_EVENT: &str = "matrix-timeline-view-updated";
const MAX_AGENT_CARD_JSON_BYTES: usize = 200_000;
const AGENT_CARD_CONTENT_KEYS: [&str; 4] = [
    "org.hermes.agent",
    "io.hermes.agent",
    "in.synara.agent",
    "m.custom.agent",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineViewPosition {
    LiveBottom,
    Unread { anchor_event_id: EventId },
    Focused { target_event_id: EventId },
    Restored { anchor_event_id: Option<EventId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelinePageState {
    Available,
    Exhausted,
    Loading,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePaginationState {
    pub backward: TimelinePageState,
    pub forward: TimelinePageState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReadState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_read_event_id: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread_anchor_event_id: Option<EventId>,
    pub is_marked_unread: bool,
}

/// Opaque reference resolved only by a bounded native media protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMediaHandle {
    pub handle_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReaction {
    pub key: String,
    pub count: u32,
    /// `None` until the native snapshot owns the active-user context. A
    /// presenter must not represent unknown ownership as an unreacted state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReplyPreview {
    pub event_id: EventId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<UserId>,
    pub sender_name: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineThreadSummary {
    pub root_event_id: EventId,
    pub reply_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_event_id: Option<EventId>,
}

/// Per-row affordances. `false` means the presenter must not render the
/// action; a missing native command is never represented as a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRowCapabilities {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineViewCapabilities {
    pub mark_read: bool,
    pub mark_unread: bool,
    pub paginate_backward: bool,
    pub paginate_forward: bool,
}

/// Current room-power authorization used to project server-mutating row
/// affordances. Absence or a failed power-level read maps to `default()` and
/// therefore withdraws every privileged capability.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimelineRoomActionAuthority {
    pub can_pin_events: bool,
    pub can_redact_own: bool,
    pub can_redact_other: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEventRowBase {
    pub item_id: TimelineItemId,
    /// Absent only for a local echo which has not received a server event ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    pub sender_id: UserId,
    pub sender_name: String,
    /// Optional `mxc://` avatar for the sender. Absent when the profile is
    /// unresolved; presenters fall back to initials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_avatar_url: Option<String>,
    pub origin_server_ts: u64,
    pub capabilities: TimelineRowCapabilities,
}

/// Project the SDK-owned metadata common to every event row.
///
/// The sender ID is the safe fallback display label until the native profile
/// projection supplies a resolved display name. Affordance gates follow the
/// typed native command surface: reply/edit/redact/forward/react open only when
/// those owners exist. Reactions consume the merged V-SEND.2 commands.
pub fn project_event_row_base(item_id: &str, event: &EventTimelineItem) -> TimelineEventRowBase {
    project_event_row_base_for_user(item_id, event, None, TimelineRoomActionAuthority::default())
}

fn project_event_row_base_for_user(
    item_id: &str,
    event: &EventTimelineItem,
    own_user_id: Option<&RumaUserId>,
    authority: TimelineRoomActionAuthority,
) -> TimelineEventRowBase {
    let sender_id = event.sender().to_string();
    let (sender_name, sender_avatar_url) = project_sender_presentation(event);
    TimelineEventRowBase {
        item_id: item_id.to_owned(),
        event_id: event.event_id().map(ToString::to_string),
        sender_name,
        sender_id,
        sender_avatar_url,
        origin_server_ts: event.timestamp().get().into(),
        capabilities: project_row_action_capabilities(event, own_user_id, authority),
    }
}

fn project_sender_presentation(event: &EventTimelineItem) -> (String, Option<String>) {
    let sender_id = event.sender().as_str();
    match event.sender_profile() {
        TimelineDetails::Ready(profile) => (
            profile
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| sender_localpart_or_id(sender_id)),
            profile.avatar_url.as_ref().map(ToString::to_string),
        ),
        _ => (sender_localpart_or_id(sender_id), None),
    }
}

fn sender_localpart_or_id(sender_id: &str) -> String {
    sender_id
        .strip_prefix('@')
        .and_then(|rest| rest.split_once(':').map(|(local, _)| local))
        .filter(|local| !local.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| sender_id.to_owned())
}

fn project_row_action_capabilities(
    event: &EventTimelineItem,
    own_user_id: Option<&RumaUserId>,
    authority: TimelineRoomActionAuthority,
) -> TimelineRowCapabilities {
    let has_remote_id = event.event_id().is_some();
    let is_redacted = event.content().is_redacted();
    let forwardable = matches!(
        event.content(),
        TimelineItemContent::MsgLike(content)
            if matches!(
                content.kind,
                MsgLikeKind::Message(_) | MsgLikeKind::Sticker(_)
            )
    );
    let reactable = matches!(
        event.content(),
        TimelineItemContent::MsgLike(content)
            if matches!(
                content.kind,
                MsgLikeKind::Message(_) | MsgLikeKind::Sticker(_) | MsgLikeKind::Poll(_)
            )
    );
    let voteable = matches!(
        event.content(),
        TimelineItemContent::MsgLike(content)
            if matches!(&content.kind, MsgLikeKind::Poll(poll) if poll.results().end_time.is_none())
    );
    let declineable = match event.content() {
        TimelineItemContent::RtcNotification { declined_by, .. } => {
            rtc_can_decline(event.is_own(), own_user_id, declined_by)
        }
        _ => false,
    };
    TimelineRowCapabilities {
        // V-SEND.2 reaction toggle/ensure/redact is on the integration tip.
        react: has_remote_id && reactable && own_user_id.is_some(),
        reply: has_remote_id && forwardable,
        edit: event.is_editable(),
        // Moderator redaction remains hidden until Core projects an
        // authoritative room-power decision; own non-redacted events are safe.
        redact: can_offer_redact(has_remote_id, event.is_own(), is_redacted, authority),
        report: has_remote_id && !event.is_own(),
        pin: can_offer_pin(has_remote_id, authority.can_pin_events, is_redacted),
        forward: has_remote_id && forwardable,
        vote: has_remote_id && voteable,
        decline_call: has_remote_id && declineable,
    }
}

fn can_offer_redact(
    has_remote_id: bool,
    event_is_own: bool,
    is_redacted: bool,
    authority: TimelineRoomActionAuthority,
) -> bool {
    has_remote_id
        && !is_redacted
        && if event_is_own {
            authority.can_redact_own
        } else {
            authority.can_redact_other
        }
}

fn can_offer_pin(has_remote_id: bool, can_pin_events: bool, is_redacted: bool) -> bool {
    has_remote_id && can_pin_events && !is_redacted
}

fn rtc_can_decline(
    event_is_own: bool,
    own_user_id: Option<&RumaUserId>,
    declined_by: &[matrix_sdk::ruma::OwnedUserId],
) -> bool {
    !event_is_own
        && own_user_id
            .is_some_and(|user_id| !declined_by.iter().any(|declined| declined == user_id))
}

fn poll_has_vote_controls(is_closed: bool, max_selections: u64, answer_count: usize) -> bool {
    !is_closed && max_selections > 0 && answer_count > 0
}

pub fn project_event_row(item_id: &str, event: &EventTimelineItem) -> TimelineViewRow {
    project_event_row_for_user(
        item_id,
        event,
        None,
        TimelineRoomActionAuthority::default(),
        None,
    )
}

fn project_event_row_for_user(
    item_id: &str,
    event: &EventTimelineItem,
    own_user_id: Option<&RumaUserId>,
    authority: TimelineRoomActionAuthority,
    mut media_registry: Option<&mut TimelineMediaRegistry>,
) -> TimelineViewRow {
    let mut base = project_event_row_base_for_user(item_id, event, own_user_id, authority);
    match event.content() {
        TimelineItemContent::MsgLike(content) => match &content.kind {
            MsgLikeKind::Message(message) => {
                let msgtype = message.msgtype();
                let forward_transport = project_forward_transport(msgtype);
                // A generic room-message body is not proof that flattening the
                // event to `m.text` preserves its semantics (for example
                // location, verification, server-notice, or custom messages).
                // Core therefore withdraws the capability unless it also owns
                // an exact supported transport.
                if forward_transport.is_none() {
                    base.capabilities.forward = false;
                }
                let (message_type, media) =
                    project_message_type_and_media(item_id, msgtype, media_registry.as_deref_mut());
                let (media_filename, media_caption) = project_media_filename_and_caption(msgtype);
                let body = media_filename
                    .as_ref()
                    .map(|_| media_caption.clone().unwrap_or_default())
                    .unwrap_or_else(|| message.body().to_owned());
                let is_agent_approval = own_user_id.is_some_and(|current_user_id| {
                    is_eligible_agent_approval_prompt(
                        &body,
                        &base.sender_id,
                        current_user_id.as_str(),
                    )
                });
                TimelineViewRow::Message(Box::new(TimelineMessageRow {
                    event: base,
                    body,
                    formatted_body: project_formatted_body(msgtype),
                    agent_card_json: project_agent_card_json(event, message.body()),
                    is_agent_approval,
                    message_type,
                    forward_transport,
                    media_filename,
                    media_caption,
                    edited: message.is_edited(),
                    reply: project_reply(content),
                    thread_root: content.thread_root.as_ref().map(ToString::to_string),
                    thread: project_thread_summary(content, event),
                    reactions: project_reactions(content, own_user_id),
                    media,
                }))
            }
            MsgLikeKind::Poll(poll) => {
                let results = poll.results();
                base.capabilities.vote &= poll_has_vote_controls(
                    results.end_time.is_some(),
                    results.max_selections,
                    results.answers.len(),
                );
                TimelineViewRow::Poll(TimelinePollRow {
                    event: base,
                    question: results.question,
                    closed: results.end_time.is_some(),
                    max_selections: u32::try_from(results.max_selections).unwrap_or(u32::MAX),
                    answers: project_poll_answers(
                        results
                            .answers
                            .into_iter()
                            .map(|answer| (answer.id, answer.text)),
                        &results.votes,
                        own_user_id.map(RumaUserId::as_str),
                    ),
                    reply: project_reply(content),
                    thread_root: content.thread_root.as_ref().map(ToString::to_string),
                    thread: project_thread_summary(content, event),
                    reactions: project_reactions(content, own_user_id),
                })
            }
            MsgLikeKind::Redacted => TimelineViewRow::Redacted(TimelineRedactedRow {
                event: base,
                summary: "Message removed".to_owned(),
            }),
            MsgLikeKind::UnableToDecrypt(_) => {
                TimelineViewRow::EncryptedUnavailable(TimelineEncryptedUnavailableRow {
                    event: base,
                    reason_code: "unable_to_decrypt".to_owned(),
                })
            }
            MsgLikeKind::Sticker(sticker) => {
                let sticker_content = sticker.content();
                let media = media_registry.and_then(|registry| {
                    registry.register(
                        item_id,
                        sticker_content.source.clone().into(),
                        sticker_content.info.mimetype.clone(),
                        sticker_content
                            .info
                            .width
                            .and_then(|value| u32::try_from(u64::from(value)).ok()),
                        sticker_content
                            .info
                            .height
                            .and_then(|value| u32::try_from(u64::from(value)).ok()),
                        None,
                    )
                });
                match media {
                    Some(media) => TimelineViewRow::Sticker {
                        event: base,
                        media,
                        forward_transport: TimelineForwardTransport::Media,
                        reply: project_reply(content),
                        thread_root: content.thread_root.as_ref().map(ToString::to_string),
                        thread: project_thread_summary(content, event),
                        reactions: project_reactions(content, own_user_id),
                    },
                    None => other_event_row(
                        base,
                        None,
                        Some(TimelineForwardTransport::Media),
                        "Sticker unavailable",
                    ),
                }
            }
            _ => other_event_row(base, None, None, "Unsupported timeline event"),
        },
        TimelineItemContent::MembershipChange(change) => {
            TimelineViewRow::Membership(TimelineMembershipRow {
                event: base,
                target_user_id: change.user_id().to_string(),
                summary: membership_change_summary(change),
            })
        }
        TimelineItemContent::ProfileChange(change) => TimelineViewRow::State(TimelineStateRow {
            event: base,
            state_type: "m.room.member".to_owned(),
            summary: profile_change_summary(change),
        }),
        TimelineItemContent::OtherState(change) => TimelineViewRow::State(TimelineStateRow {
            event: base,
            state_type: change.content().event_type().to_string(),
            summary: other_state_summary(change),
        }),
        TimelineItemContent::CallInvite => TimelineViewRow::Call(TimelineCallRow {
            event: base,
            call_kind: "invite".to_owned(),
        }),
        TimelineItemContent::RtcNotification { .. } => TimelineViewRow::Call(TimelineCallRow {
            event: base,
            call_kind: "notification".to_owned(),
        }),
        TimelineItemContent::FailedToParseMessageLike { .. }
        | TimelineItemContent::FailedToParseState { .. } => {
            other_event_row(base, None, None, "Unsupported timeline event")
        }
    }
}

/// Project poll answer options with counts only (no voter user IDs over IPC).
pub fn project_poll_answers(
    answers: impl IntoIterator<Item = (String, String)>,
    votes: &HashMap<String, Vec<String>>,
    own_user_id: Option<&str>,
) -> Vec<TimelinePollAnswer> {
    answers
        .into_iter()
        .map(|(id, text)| {
            let voters = votes.get(&id).map(Vec::as_slice).unwrap_or(&[]);
            let vote_count = u32::try_from(voters.len()).unwrap_or(u32::MAX);
            let own =
                own_user_id.is_some_and(|user_id| voters.iter().any(|voter| voter == user_id));
            TimelinePollAnswer {
                id,
                text,
                vote_count,
                own,
            }
        })
        .collect()
}

/// Preserve distinct Matrix `formatted_body` protocol content for presenters.
///
/// The returned HTML remains untrusted. Every platform presenter must apply
/// its output-context sanitizer and bounded parser before rendering it.
pub fn project_formatted_body(msgtype: &MessageType) -> Option<String> {
    let formatted = match msgtype {
        MessageType::Text(content) => content.formatted.as_ref(),
        MessageType::Notice(content) => content.formatted.as_ref(),
        MessageType::Emote(content) => content.formatted.as_ref(),
        MessageType::Image(content) => content.formatted_caption(),
        MessageType::File(content) => content.formatted_caption(),
        MessageType::Audio(content) => content.formatted_caption(),
        MessageType::Video(content) => content.formatted_caption(),
        _ => None,
    }?;
    if formatted.format != MessageFormat::Html {
        return None;
    }
    let html = formatted.body.trim();
    if html.is_empty() || html == msgtype.body().trim() {
        return None;
    }
    Some(html.to_owned())
}

/// Keep Matrix media filenames and captions distinct. The legacy `body`
/// field is ambiguous for media because pre-caption events store the filename
/// there, while captioned events store the caption and move the filename to
/// `filename`.
pub fn project_media_filename_and_caption(
    msgtype: &MessageType,
) -> (Option<String>, Option<String>) {
    let fields = match msgtype {
        MessageType::Image(content) => (content.filename(), content.caption()),
        MessageType::File(content) => (content.filename(), content.caption()),
        MessageType::Audio(content) => (content.filename(), content.caption()),
        MessageType::Video(content) => (content.filename(), content.caption()),
        _ => return (None, None),
    };
    (
        Some(fields.0.to_owned()),
        fields
            .1
            .map(str::to_owned)
            .filter(|caption| !caption.is_empty()),
    )
}

/// Project only the recognized structured agent-card object from a message.
///
/// The native boundary intentionally does not expose arbitrary Matrix event
/// JSON. Direct custom-content fields are preferred; the body wrapper remains
/// supported for compatibility with existing Hermes agents and encrypted
/// messages whose decrypted custom content is represented by the SDK body.
fn project_agent_card_json(event: &EventTimelineItem, body: &str) -> Option<String> {
    let direct = event
        .latest_json()
        .filter(|raw| raw.json().get().len() <= MAX_AGENT_CARD_JSON_BYTES * 2)
        .and_then(|raw| serde_json::from_str::<JsonValue>(raw.json().get()).ok())
        .and_then(|event| event.get("content").cloned())
        .and_then(|content| agent_card_payload_from_content(&content));

    direct
        .or_else(|| agent_card_payload_from_body(body))
        .and_then(|payload| serialize_bounded_agent_card(&payload))
}

fn agent_card_payload_from_content(content: &JsonValue) -> Option<JsonValue> {
    let object = content.as_object()?;
    AGENT_CARD_CONTENT_KEYS
        .iter()
        .find_map(|key| object.get(*key).filter(|value| value.is_object()).cloned())
}

fn agent_card_payload_from_body(body: &str) -> Option<JsonValue> {
    if body.len() > MAX_AGENT_CARD_JSON_BYTES {
        return None;
    }
    let parsed = serde_json::from_str::<JsonValue>(body).ok()?;
    if let Some(payload) = agent_card_payload_from_content(&parsed) {
        return Some(payload);
    }
    let object = parsed.as_object()?;
    if object.get("hermes").and_then(JsonValue::as_bool) != Some(true) {
        return None;
    }
    object
        .get("payload")
        .or_else(|| object.get("agent"))
        .filter(|value| value.is_object())
        .cloned()
}

fn serialize_bounded_agent_card(payload: &JsonValue) -> Option<String> {
    let encoded = serde_json::to_string(payload).ok()?;
    (encoded.len() <= MAX_AGENT_CARD_JSON_BYTES).then_some(encoded)
}

pub fn project_message_type_and_media(
    item_id: &str,
    msgtype: &MessageType,
    media_registry: Option<&mut TimelineMediaRegistry>,
) -> (Option<String>, Option<TimelineMediaHandle>) {
    match msgtype {
        MessageType::Text(_) => (Some("text".to_owned()), None),
        MessageType::Notice(_) => (Some("notice".to_owned()), None),
        MessageType::Emote(_) => (Some("emote".to_owned()), None),
        MessageType::Image(content) => (
            Some("image".to_owned()),
            media_registry.and_then(|registry| {
                registry.register(
                    item_id,
                    content.source.clone(),
                    content.info.as_ref().and_then(|info| info.mimetype.clone()),
                    content
                        .info
                        .as_ref()
                        .and_then(|info| info.width)
                        .and_then(|value| u32::try_from(u64::from(value)).ok()),
                    content
                        .info
                        .as_ref()
                        .and_then(|info| info.height)
                        .and_then(|value| u32::try_from(u64::from(value)).ok()),
                    None,
                )
            }),
        ),
        MessageType::File(content) => (
            Some("file".to_owned()),
            media_registry.and_then(|registry| {
                registry.register(
                    item_id,
                    content.source.clone(),
                    content.info.as_ref().and_then(|info| info.mimetype.clone()),
                    None,
                    None,
                    None,
                )
            }),
        ),
        MessageType::Audio(content) => (
            Some("audio".to_owned()),
            media_registry.and_then(|registry| {
                registry.register(
                    item_id,
                    content.source.clone(),
                    content.info.as_ref().and_then(|info| info.mimetype.clone()),
                    None,
                    None,
                    content
                        .info
                        .as_ref()
                        .and_then(|info| info.duration)
                        .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
                )
            }),
        ),
        MessageType::Video(content) => (
            Some("video".to_owned()),
            media_registry.and_then(|registry| {
                registry.register(
                    item_id,
                    content.source.clone(),
                    content.info.as_ref().and_then(|info| info.mimetype.clone()),
                    content
                        .info
                        .as_ref()
                        .and_then(|info| info.width)
                        .and_then(|value| u32::try_from(u64::from(value)).ok()),
                    content
                        .info
                        .as_ref()
                        .and_then(|info| info.height)
                        .and_then(|value| u32::try_from(u64::from(value)).ok()),
                    content
                        .info
                        .as_ref()
                        .and_then(|info| info.duration)
                        .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
                )
            }),
        ),
        _ => (None, None),
    }
}

/// Select the only Core action owner capable of faithfully forwarding this
/// Matrix message. This remains stable even when the bounded media registry
/// cannot currently allocate a presentation/download handle.
pub fn project_forward_transport(msgtype: &MessageType) -> Option<TimelineForwardTransport> {
    match msgtype {
        MessageType::Image(_)
        | MessageType::File(_)
        | MessageType::Audio(_)
        | MessageType::Video(_) => Some(TimelineForwardTransport::Media),
        MessageType::Text(_) | MessageType::Notice(_) | MessageType::Emote(_) => {
            Some(TimelineForwardTransport::Text)
        }
        _ => None,
    }
}

fn membership_change_summary(change: &RoomMembershipChange) -> String {
    let target = change
        .display_name()
        .unwrap_or_else(|| change.user_id().to_string());
    match change.change() {
        Some(MembershipChange::Joined) => format!("{target} joined the room"),
        Some(MembershipChange::Left) => format!("{target} left the room"),
        Some(MembershipChange::Banned) => format!("{target} was banned"),
        Some(MembershipChange::Unbanned) => format!("{target} was unbanned"),
        Some(MembershipChange::Kicked) => format!("{target} was kicked"),
        Some(MembershipChange::Invited) => format!("{target} was invited"),
        Some(MembershipChange::KickedAndBanned) => format!("{target} was kicked and banned"),
        Some(MembershipChange::InvitationAccepted) => format!("{target} accepted the invite"),
        Some(MembershipChange::InvitationRejected) => format!("{target} rejected the invite"),
        Some(MembershipChange::InvitationRevoked) => format!("{target}'s invite was revoked"),
        Some(MembershipChange::Knocked) => format!("{target} requested to join"),
        Some(MembershipChange::KnockAccepted) => format!("{target}'s knock was accepted"),
        Some(MembershipChange::KnockRetracted) => format!("{target} retracted their knock"),
        Some(MembershipChange::KnockDenied) => format!("{target}'s knock was denied"),
        Some(MembershipChange::None)
        | Some(MembershipChange::Error)
        | Some(MembershipChange::NotImplemented)
        | None => {
            format!("{target} membership changed")
        }
    }
}

fn profile_change_summary(change: &MemberProfileChange) -> String {
    let user = change.user_id();
    match (
        change.displayname_change(),
        change.avatar_url_change().is_some(),
    ) {
        (Some(name_change), true) => match (&name_change.old, &name_change.new) {
            (Some(old), Some(new)) if old != new => {
                format!("{old} is now {new} (avatar updated)")
            }
            (_, Some(new)) => format!("{new} updated their profile"),
            _ => format!("{user} updated their profile"),
        },
        (Some(name_change), false) => match (&name_change.old, &name_change.new) {
            (Some(old), Some(new)) if old != new => format!("{old} is now {new}"),
            (_, Some(new)) => format!("{new} set a display name"),
            (Some(old), None) => format!("{old} cleared their display name"),
            _ => format!("{user} updated their display name"),
        },
        (None, true) => format!("{user} updated their avatar"),
        (None, false) => format!("Profile updated for {user}"),
    }
}

fn other_state_summary(change: &OtherState) -> String {
    match change.content() {
        AnyOtherStateEventContentChange::RoomName(content) => match content {
            StateEventContentChange::Original { content, .. } => {
                let name = content.name.trim();
                if name.is_empty() {
                    "Room name cleared".to_owned()
                } else {
                    format!("Room name set to {name}")
                }
            }
            StateEventContentChange::Redacted(_) => "Room name removed".to_owned(),
        },
        AnyOtherStateEventContentChange::RoomTopic(content) => match content {
            StateEventContentChange::Original { content, .. } => {
                let topic = content.topic.trim();
                if topic.is_empty() {
                    "Room topic cleared".to_owned()
                } else {
                    format!("Room topic set to {topic}")
                }
            }
            StateEventContentChange::Redacted(_) => "Room topic removed".to_owned(),
        },
        AnyOtherStateEventContentChange::RoomAvatar(_) => "Room avatar updated".to_owned(),
        AnyOtherStateEventContentChange::RoomCanonicalAlias(_) => "Room address updated".to_owned(),
        AnyOtherStateEventContentChange::RoomCreate(_) => "Room created".to_owned(),
        AnyOtherStateEventContentChange::RoomEncryption(_) => {
            "Encryption enabled for this room".to_owned()
        }
        AnyOtherStateEventContentChange::RoomGuestAccess(_) => "Guest access updated".to_owned(),
        AnyOtherStateEventContentChange::RoomHistoryVisibility(_) => {
            "History visibility updated".to_owned()
        }
        AnyOtherStateEventContentChange::RoomJoinRules(_) => "Join rules updated".to_owned(),
        AnyOtherStateEventContentChange::RoomPinnedEvents(_) => {
            "Pinned messages updated".to_owned()
        }
        AnyOtherStateEventContentChange::RoomPowerLevels(_) => "Power levels updated".to_owned(),
        AnyOtherStateEventContentChange::RoomTombstone(_) => "Room upgraded".to_owned(),
        AnyOtherStateEventContentChange::RoomServerAcl(_) => "Server ACL updated".to_owned(),
        AnyOtherStateEventContentChange::RoomThirdPartyInvite(_) => {
            "Third-party invite updated".to_owned()
        }
        AnyOtherStateEventContentChange::SpaceChild(_)
        | AnyOtherStateEventContentChange::SpaceParent(_) => "Space relation updated".to_owned(),
        AnyOtherStateEventContentChange::PolicyRuleRoom(_)
        | AnyOtherStateEventContentChange::PolicyRuleServer(_)
        | AnyOtherStateEventContentChange::PolicyRuleUser(_) => {
            "Moderation policy updated".to_owned()
        }
        AnyOtherStateEventContentChange::_Custom { event_type } => {
            format!("Room state updated ({event_type})")
        }
    }
}

fn project_reactions(
    content: &matrix_sdk_ui::timeline::MsgLikeContent,
    own_user_id: Option<&RumaUserId>,
) -> Vec<TimelineReaction> {
    content
        .reactions
        .iter()
        .map(|(key, reactions)| TimelineReaction {
            key: key.clone(),
            count: reactions.len().try_into().unwrap_or(u32::MAX),
            own: own_user_id.map(|user_id| reactions.contains_key(user_id)),
        })
        .collect()
}

fn project_reply(
    content: &matrix_sdk_ui::timeline::MsgLikeContent,
) -> Option<TimelineReplyPreview> {
    let details = content.in_reply_to.as_ref()?;
    match &details.event {
        TimelineDetails::Ready(event) => {
            let body = event
                .content
                .as_message()
                .map(|message| message.body().to_owned())
                .filter(|body| !body.trim().is_empty())
                .unwrap_or_else(|| "Original message".to_owned());
            Some(TimelineReplyPreview {
                event_id: details.event_id.to_string(),
                sender_id: Some(event.sender.to_string()),
                sender_name: sender_localpart_or_id(event.sender.as_str()),
                body,
            })
        }
        TimelineDetails::Unavailable | TimelineDetails::Pending | TimelineDetails::Error(_) => {
            Some(TimelineReplyPreview {
                event_id: details.event_id.to_string(),
                sender_id: None,
                sender_name: "Message".to_owned(),
                body: "Jump to original".to_owned(),
            })
        }
    }
}

fn project_thread_summary(
    content: &matrix_sdk_ui::timeline::MsgLikeContent,
    event: &EventTimelineItem,
) -> Option<TimelineThreadSummary> {
    let summary = content.thread_summary.as_ref()?;
    let root_event_id = event.event_id()?.to_string();
    // Project a remote event id only. Local-echo transaction ids and pending
    // embeds stay absent rather than inventing a JS-side id.
    let latest_event_id = match &summary.latest_event {
        TimelineDetails::Ready(embedded) => match &embedded.identifier {
            TimelineEventItemId::EventId(event_id) => Some(event_id.to_string()),
            TimelineEventItemId::TransactionId(_) => None,
        },
        TimelineDetails::Unavailable | TimelineDetails::Pending | TimelineDetails::Error(_) => None,
    };
    Some(TimelineThreadSummary {
        root_event_id,
        reply_count: summary.num_replies,
        latest_event_id,
    })
}

/// Project one SDK item without allowing the SDK object graph to cross the
/// presenter boundary. The SDK supplies only three virtual item kinds; native
/// unread and pagination rows are synthesized later by their respective
/// read-frontier and pagination owners.
pub fn project_timeline_item(
    item: &SdkTimelineItem,
    own_user_id: Option<&RumaUserId>,
) -> TimelineViewRow {
    let item_id = item.unique_id().0.clone();
    if let Some(event) = item.as_event() {
        return project_event_row_for_user(
            &item_id,
            event,
            own_user_id,
            TimelineRoomActionAuthority::default(),
            None,
        );
    }

    match item.as_virtual() {
        Some(VirtualTimelineItem::DateDivider(timestamp)) => TimelineViewRow::DateSeparator {
            item_id,
            // The renderer formats this neutral instant for its locale. Rust
            // must not invent a day key using a machine-local timezone.
            timestamp_ms: timestamp.get().into(),
        },
        Some(VirtualTimelineItem::ReadMarker) => TimelineViewRow::ReadMarker { item_id },
        Some(VirtualTimelineItem::TimelineStart) => TimelineViewRow::TimelineStart { item_id },
        None => other_row(&item_id, None, "Unsupported timeline item"),
    }
}

/// Project one SDK item while registering any native media source in the
/// exact opened stream that owns the resulting row.
pub fn project_timeline_item_with_media(
    item: &SdkTimelineItem,
    own_user_id: Option<&RumaUserId>,
    authority: TimelineRoomActionAuthority,
    media_registry: &mut TimelineMediaRegistry,
) -> TimelineViewRow {
    let item_id = item.unique_id().0.clone();
    if let Some(event) = item.as_event() {
        return project_event_row_for_user(
            &item_id,
            event,
            own_user_id,
            authority,
            Some(media_registry),
        );
    }
    project_timeline_item(item, own_user_id)
}

fn other_row(item_id: &str, event_id: Option<EventId>, summary: &str) -> TimelineViewRow {
    TimelineViewRow::Other(TimelineOtherRow {
        item_id: item_id.to_owned(),
        event_id,
        event: None,
        event_type: None,
        forward_transport: None,
        summary: summary.to_owned(),
    })
}

fn other_event_row(
    event: TimelineEventRowBase,
    event_type: Option<String>,
    forward_transport: Option<TimelineForwardTransport>,
    summary: &str,
) -> TimelineViewRow {
    TimelineViewRow::Other(TimelineOtherRow {
        item_id: event.item_id.clone(),
        event_id: event.event_id.clone(),
        event: Some(event),
        event_type,
        forward_transport,
        summary: summary.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMessageRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub body: String,
    /// Untrusted Matrix `formatted_body` protocol content. Presenters must
    /// apply an output-context sanitizer before rendering it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_body: Option<String>,
    /// Recognized, size-bounded Synara/Hermes card payload only. This is never
    /// the complete raw Matrix event or content object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_card_json: Option<String>,
    /// Core-owned approval eligibility. Presenters may parse the body for
    /// display details only after this authoritative gate is true.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_agent_approval: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    /// Core-owned dispatch route for forwarding this semantic event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_transport: Option<TimelineForwardTransport>,
    /// Matrix media filename, never inferred from `body` by presenters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_filename: Option<String>,
    /// Plain Matrix media caption. Formatted markup stays in `formatted_body`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_caption: Option<String>,
    pub edited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<TimelineReplyPreview>,
    /// Event ID of the thread this event belongs to. This is independent of
    /// `reply`, whose target may be another child within the same thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_root: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<TimelineThreadSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<TimelineReaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<TimelineMediaHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineForwardTransport {
    Text,
    Media,
}

impl TimelineForwardTransport {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Media => "media",
        }
    }
}

/// One poll answer option. Vote tallies are counts only — never voter user IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePollAnswer {
    pub id: String,
    pub text: String,
    pub vote_count: u32,
    /// Whether the active native session has selected this answer.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePollRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub question: String,
    pub closed: bool,
    /// Maximum simultaneous selections the poll allows (MSC3381).
    pub max_selections: u32,
    pub answers: Vec<TimelinePollAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<TimelineReplyPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_root: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<TimelineThreadSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<TimelineReaction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMembershipRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub target_user_id: UserId,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineStateRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub state_type: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineCallRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub call_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRedactedRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEncryptedUnavailableRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineOtherRow {
    pub item_id: TimelineItemId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    /// Present for SDK event rows. Virtual/unsupported placeholders have no
    /// sender, timestamp, or event capabilities to transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<TimelineEventRowBase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_transport: Option<TimelineForwardTransport>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TimelineViewRow {
    Message(Box<TimelineMessageRow>),
    Sticker {
        event: TimelineEventRowBase,
        media: TimelineMediaHandle,
        forward_transport: TimelineForwardTransport,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reply: Option<TimelineReplyPreview>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thread_root: Option<EventId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thread: Option<TimelineThreadSummary>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        reactions: Vec<TimelineReaction>,
    },
    Poll(TimelinePollRow),
    Membership(TimelineMembershipRow),
    State(TimelineStateRow),
    Call(TimelineCallRow),
    Redacted(TimelineRedactedRow),
    EncryptedUnavailable(TimelineEncryptedUnavailableRow),
    Other(TimelineOtherRow),
    DateSeparator {
        item_id: TimelineItemId,
        /// The SDK separator's local-day instant in milliseconds. Locale
        /// rendering belongs to the SDK-neutral presenter, not the Rust host.
        timestamp_ms: u64,
    },
    ReadMarker {
        item_id: TimelineItemId,
    },
    UnreadMarker {
        item_id: TimelineItemId,
    },
    TimelineStart {
        item_id: TimelineItemId,
    },
    Pagination {
        item_id: TimelineItemId,
        direction: String,
        state: TimelinePageState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineViewSnapshot {
    pub schema_version: u32,
    pub session_generation: u64,
    pub room_id: RoomId,
    pub revision: u64,
    pub position: TimelineViewPosition,
    pub pagination: TimelinePaginationState,
    pub read_state: TimelineReadState,
    /// Authoritative `m.room.pinned_events` ids for this room (empty when none).
    /// Presenters gate Pin vs Unpin against this list; do not invent pin state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pinned_event_ids: Vec<EventId>,
    pub rows: Vec<TimelineViewRow>,
    pub capabilities: TimelineViewCapabilities,
}

/// One ordered native update to a `TimelineViewSnapshot` row list. This mirrors
/// every SDK `VectorDiff` variant so the presenter never has to infer or poll
/// a missing mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TimelineViewDeltaOp {
    Append { rows: Vec<TimelineViewRow> },
    Clear,
    PushFront { row: TimelineViewRow },
    PushBack { row: TimelineViewRow },
    PopFront,
    PopBack,
    Insert { index: usize, row: TimelineViewRow },
    Set { index: usize, row: TimelineViewRow },
    Remove { index: usize },
    Truncate { len: usize },
    Reset { rows: Vec<TimelineViewRow> },
}

/// A native timeline update emitted only from the managed SDK subscription.
///
/// Row mutations travel in `ops`. Live read-frontier and pagination owners may
/// also attach authoritative metadata; metadata-only batches are valid when
/// `ops` is empty but at least one of `read_state` / `pagination` is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineViewDeltaBatch {
    pub schema_version: u32,
    pub session_generation: u64,
    /// Matches `NativeTimelineOpenReadback.stream_id`; required to keep a
    /// room's live and focused views from consuming one another's updates.
    pub stream_id: String,
    pub room_id: RoomId,
    pub revision: u64,
    pub ops: Vec<TimelineViewDeltaOp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_state: Option<TimelineReadState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<TimelinePaginationState>,
    /// Full replacement of room pin state when the native owner observes a change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_event_ids: Option<Vec<EventId>>,
}

/// Project one SDK delta batch while keeping SDK items and diffs in Rust.
pub fn project_timeline_diffs(
    diffs: &[VectorDiff<Arc<SdkTimelineItem>>],
    own_user_id: Option<&RumaUserId>,
) -> Vec<TimelineViewDeltaOp> {
    diffs
        .iter()
        .map(|diff| match diff {
            VectorDiff::Append { values } => TimelineViewDeltaOp::Append {
                rows: values
                    .iter()
                    .map(|item| project_timeline_item(item, own_user_id))
                    .collect(),
            },
            VectorDiff::Clear => TimelineViewDeltaOp::Clear,
            VectorDiff::PushFront { value } => TimelineViewDeltaOp::PushFront {
                row: project_timeline_item(value, own_user_id),
            },
            VectorDiff::PushBack { value } => TimelineViewDeltaOp::PushBack {
                row: project_timeline_item(value, own_user_id),
            },
            VectorDiff::PopFront => TimelineViewDeltaOp::PopFront,
            VectorDiff::PopBack => TimelineViewDeltaOp::PopBack,
            VectorDiff::Insert { index, value } => TimelineViewDeltaOp::Insert {
                index: *index,
                row: project_timeline_item(value, own_user_id),
            },
            VectorDiff::Set { index, value } => TimelineViewDeltaOp::Set {
                index: *index,
                row: project_timeline_item(value, own_user_id),
            },
            VectorDiff::Remove { index } => TimelineViewDeltaOp::Remove { index: *index },
            VectorDiff::Truncate { length } => TimelineViewDeltaOp::Truncate { len: *length },
            VectorDiff::Reset { values } => TimelineViewDeltaOp::Reset {
                rows: values
                    .iter()
                    .map(|item| project_timeline_item(item, own_user_id))
                    .collect(),
            },
        })
        .collect()
}

pub fn project_timeline_diffs_with_media(
    diffs: &[VectorDiff<Arc<SdkTimelineItem>>],
    own_user_id: Option<&RumaUserId>,
    authority: TimelineRoomActionAuthority,
    media_registry: &mut TimelineMediaRegistry,
) -> Vec<TimelineViewDeltaOp> {
    diffs
        .iter()
        .map(|diff| match diff {
            VectorDiff::Append { values } => TimelineViewDeltaOp::Append {
                rows: values
                    .iter()
                    .map(|item| {
                        project_timeline_item_with_media(
                            item,
                            own_user_id,
                            authority,
                            media_registry,
                        )
                    })
                    .collect(),
            },
            VectorDiff::Clear => TimelineViewDeltaOp::Clear,
            VectorDiff::PushFront { value } => TimelineViewDeltaOp::PushFront {
                row: project_timeline_item_with_media(
                    value,
                    own_user_id,
                    authority,
                    media_registry,
                ),
            },
            VectorDiff::PushBack { value } => TimelineViewDeltaOp::PushBack {
                row: project_timeline_item_with_media(
                    value,
                    own_user_id,
                    authority,
                    media_registry,
                ),
            },
            VectorDiff::PopFront => TimelineViewDeltaOp::PopFront,
            VectorDiff::PopBack => TimelineViewDeltaOp::PopBack,
            VectorDiff::Insert { index, value } => TimelineViewDeltaOp::Insert {
                index: *index,
                row: project_timeline_item_with_media(
                    value,
                    own_user_id,
                    authority,
                    media_registry,
                ),
            },
            VectorDiff::Set { index, value } => TimelineViewDeltaOp::Set {
                index: *index,
                row: project_timeline_item_with_media(
                    value,
                    own_user_id,
                    authority,
                    media_registry,
                ),
            },
            VectorDiff::Remove { index } => TimelineViewDeltaOp::Remove { index: *index },
            VectorDiff::Truncate { length } => TimelineViewDeltaOp::Truncate { len: *length },
            VectorDiff::Reset { values } => TimelineViewDeltaOp::Reset {
                rows: values
                    .iter()
                    .map(|item| {
                        project_timeline_item_with_media(
                            item,
                            own_user_id,
                            authority,
                            media_registry,
                        )
                    })
                    .collect(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::events::room::message::{
        EmoteMessageEventContent, ImageMessageEventContent, LocationMessageEventContent,
        NoticeMessageEventContent, TextMessageEventContent,
    };
    use matrix_sdk::ruma::user_id;

    #[test]
    fn virtual_timeline_rows_serialize_camel_case_fields() {
        let row = TimelineViewRow::DateSeparator {
            item_id: "date-1".to_owned(),
            timestamp_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_value(row).expect("date separator should serialize");
        assert_eq!(json["kind"], "date_separator");
        assert_eq!(json["itemId"], "date-1");
        assert_eq!(json["timestampMs"], 1_700_000_000_000_u64);
        assert!(json.get("item_id").is_none());
        assert!(json.get("timestamp_ms").is_none());
    }

    #[test]
    fn formatted_body_projects_distinct_html_only() {
        let rich = MessageType::Text(TextMessageEventContent::html(
            "hello",
            "<p><strong>hello</strong></p>",
        ));
        assert_eq!(
            project_formatted_body(&rich).as_deref(),
            Some("<p><strong>hello</strong></p>")
        );

        let plain = MessageType::Text(TextMessageEventContent::plain("hello"));
        assert_eq!(project_formatted_body(&plain), None);

        let same = MessageType::Notice(NoticeMessageEventContent::html("note", "note"));
        assert_eq!(project_formatted_body(&same), None);

        let emote = MessageType::Emote(EmoteMessageEventContent::html("waves", "<em>waves</em>"));
        assert_eq!(
            project_formatted_body(&emote).as_deref(),
            Some("<em>waves</em>")
        );
    }

    #[test]
    fn media_filename_and_caption_are_projected_without_body_inference() {
        let mut image = ImageMessageEventContent::plain(
            "A sunset".to_owned(),
            matrix_sdk::ruma::OwnedMxcUri::from("mxc://example.org/image"),
        );
        image.filename = Some("sunset.jpg".to_owned());
        image.formatted = Some(
            matrix_sdk::ruma::events::room::message::FormattedBody::html(
                "<strong>A sunset</strong>",
            ),
        );
        let message = MessageType::Image(image);

        assert_eq!(
            project_forward_transport(&message),
            Some(TimelineForwardTransport::Media)
        );
        assert_eq!(
            project_message_type_and_media("item", &message, None).1,
            None,
            "forward transport must not depend on a currently allocated media handle"
        );

        assert_eq!(
            project_media_filename_and_caption(&message),
            (Some("sunset.jpg".to_owned()), Some("A sunset".to_owned()))
        );
        assert_eq!(
            project_formatted_body(&message).as_deref(),
            Some("<strong>A sunset</strong>")
        );

        let bare = MessageType::Image(ImageMessageEventContent::plain(
            "bare.jpg".to_owned(),
            matrix_sdk::ruma::OwnedMxcUri::from("mxc://example.org/bare"),
        ));
        assert_eq!(
            project_media_filename_and_caption(&bare),
            (Some("bare.jpg".to_owned()), None)
        );
        assert_eq!(
            project_forward_transport(&MessageType::Text(TextMessageEventContent::plain("hi"))),
            Some(TimelineForwardTransport::Text)
        );
        assert_eq!(
            project_forward_transport(&MessageType::Notice(NoticeMessageEventContent::plain(
                "notice",
            ))),
            Some(TimelineForwardTransport::Text)
        );
        assert_eq!(
            project_forward_transport(&MessageType::Emote(EmoteMessageEventContent::plain(
                "waves",
            ))),
            Some(TimelineForwardTransport::Text)
        );
        assert_eq!(
            project_forward_transport(&MessageType::Location(LocationMessageEventContent::new(
                "location".to_owned(),
                "geo:1,2".to_owned(),
            ))),
            None,
            "semantic message types must not silently flatten into m.text"
        );
    }

    #[test]
    fn hermes_approval_html_crosses_the_shared_boundary_unchanged() {
        let html = concat!(
            "<p>⚠️ <strong>Dangerous command requires approval</strong></p>\n",
            "<pre><code>rm -rf /tmp/example\n</code></pre>\n",
            "<p>Reason: destructive command</p>\n",
            "<p>Reply <code>!approve</code> to execute, ",
            "<code>!approve session</code> to approve this pattern for the session, ",
            "<code>!approve always</code> to approve permanently, or ",
            "<code>!deny</code> to cancel.</p>\n",
            "<p>You can also react to this prompt:<br>\n",
            "✅ = approve once<br>\n♾️ = approve always<br>\n❌ = deny</p>"
        );
        let message = MessageType::Text(TextMessageEventContent::html(
            "⚠️ **Dangerous command requires approval**",
            html,
        ));

        assert_eq!(project_formatted_body(&message).as_deref(), Some(html));
    }

    #[test]
    fn agent_card_body_projection_accepts_only_recognized_bounded_objects() {
        let payload = agent_card_payload_from_body(
            r#"{"hermes":true,"payload":{"title":"Approval required","status":"pending"}}"#,
        )
        .expect("recognized Hermes card");
        assert_eq!(payload["title"], "Approval required");

        let direct = agent_card_payload_from_body(
            r#"{"in.synara.agent":{"title":"Direct card","status":"complete"}}"#,
        )
        .expect("recognized direct card");
        assert_eq!(direct["title"], "Direct card");

        assert!(
            agent_card_payload_from_body(r#"{"payload":{"title":"missing marker"}}"#).is_none()
        );
        assert!(agent_card_payload_from_body("ordinary message").is_none());
        assert!(agent_card_payload_from_body(&"x".repeat(MAX_AGENT_CARD_JSON_BYTES + 1)).is_none());
    }

    #[test]
    fn agent_card_content_projection_does_not_expose_unrecognized_event_content() {
        let content = serde_json::json!({
            "msgtype": "m.notice",
            "body": "safe fallback",
            "in.synara.agent": {"title": "Review", "status": "pending"},
            "access_token": "must-not-cross-boundary"
        });
        let payload = agent_card_payload_from_content(&content).expect("recognized direct card");
        let encoded = serialize_bounded_agent_card(&payload).expect("bounded payload");
        assert!(encoded.contains("Review"));
        assert!(!encoded.contains("access_token"));
        assert!(!encoded.contains("safe fallback"));
    }

    #[test]
    fn message_type_labels_cover_text_notice_and_emote() {
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Text(TextMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("text")
        );
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Notice(NoticeMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("notice")
        );
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Emote(EmoteMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("emote")
        );
    }

    #[test]
    fn poll_answers_project_counts_and_own_without_voter_ids() {
        let mut votes = HashMap::new();
        votes.insert(
            "a1".into(),
            vec!["@alice:example.org".into(), "@bob:example.org".into()],
        );
        votes.insert("a2".into(), vec!["@carol:example.org".into()]);

        let answers = project_poll_answers(
            [
                ("a1".into(), "Yes".into()),
                ("a2".into(), "No".into()),
                ("a3".into(), "Maybe".into()),
            ],
            &votes,
            Some("@alice:example.org"),
        );

        assert_eq!(
            answers,
            vec![
                TimelinePollAnswer {
                    id: "a1".into(),
                    text: "Yes".into(),
                    vote_count: 2,
                    own: true,
                },
                TimelinePollAnswer {
                    id: "a2".into(),
                    text: "No".into(),
                    vote_count: 1,
                    own: false,
                },
                TimelinePollAnswer {
                    id: "a3".into(),
                    text: "Maybe".into(),
                    vote_count: 0,
                    own: false,
                },
            ]
        );

        let row = TimelinePollRow {
            event: TimelineEventRowBase {
                item_id: "poll-item".into(),
                event_id: Some("$poll:example.org".into()),
                sender_id: "@alice:example.org".into(),
                sender_name: "@alice:example.org".into(),
                sender_avatar_url: None,
                origin_server_ts: 1,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: false,
                    edit: false,
                    redact: true,
                    report: false,
                    pin: true,
                    forward: false,
                    vote: true,
                    decline_call: false,
                },
            },
            question: "Lunch?".into(),
            closed: false,
            max_selections: 1,
            answers,
            reply: None,
            thread_root: Some("$root:example.org".into()),
            thread: None,
            reactions: vec![TimelineReaction {
                key: "👍".into(),
                count: 2,
                own: Some(true),
            }],
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("\"voteCount\":2"));
        assert!(json.contains("\"own\":true"));
        assert!(!json.contains("@bob:example.org"));
        assert!(!json.contains("@carol:example.org"));
        assert!(!json.contains("token"));
        assert!(!json.contains("ciphertext"));
    }

    #[test]
    fn poll_vote_controls_require_open_poll_with_positive_bound_and_answers() {
        assert!(poll_has_vote_controls(false, 1, 2));
        assert!(!poll_has_vote_controls(true, 1, 2));
        assert!(!poll_has_vote_controls(false, 0, 2));
        assert!(!poll_has_vote_controls(false, 1, 0));
    }

    #[test]
    fn rtc_decline_capability_requires_remote_undecided_event_and_known_user() {
        let own = user_id!("@me:example.org");
        assert!(rtc_can_decline(false, Some(own), &[]));
        assert!(!rtc_can_decline(true, Some(own), &[]));
        assert!(!rtc_can_decline(false, None, &[]));
        assert!(!rtc_can_decline(false, Some(own), &[own.to_owned()]));
    }

    #[test]
    fn destructive_capabilities_fail_closed_without_identity_or_room_power() {
        let moderator = TimelineRoomActionAuthority {
            can_pin_events: true,
            can_redact_own: true,
            can_redact_other: true,
        };
        let member = TimelineRoomActionAuthority {
            can_pin_events: false,
            can_redact_own: true,
            can_redact_other: false,
        };
        assert!(can_offer_redact(true, true, false, member));
        assert!(!can_offer_redact(true, false, false, member));
        assert!(can_offer_redact(true, false, false, moderator));
        assert!(!can_offer_redact(true, true, true, moderator));
        assert!(!can_offer_redact(
            true,
            true,
            false,
            TimelineRoomActionAuthority::default()
        ));
        assert!(!can_offer_redact(false, true, false, moderator));

        assert!(can_offer_pin(true, true, false));
        assert!(!can_offer_pin(true, false, false));
        assert!(!can_offer_pin(true, true, true));
        assert!(!can_offer_pin(false, true, false));
    }

    #[test]
    fn reply_and_thread_summary_serialize_product_shape_without_secrets() {
        let reply = TimelineReplyPreview {
            event_id: "$parent:example.org".into(),
            sender_id: Some("@alice:example.org".into()),
            sender_name: "alice".into(),
            body: "Earlier message".into(),
        };
        let thread = TimelineThreadSummary {
            root_event_id: "$root:example.org".into(),
            reply_count: 3,
            latest_event_id: Some("$latest:example.org".into()),
        };
        let message = TimelineMessageRow {
            event: TimelineEventRowBase {
                item_id: "msg-item".into(),
                event_id: Some("$msg:example.org".into()),
                sender_id: "@bob:example.org".into(),
                sender_name: "@bob:example.org".into(),
                sender_avatar_url: None,
                origin_server_ts: 1,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: true,
                    edit: true,
                    redact: true,
                    report: false,
                    pin: true,
                    forward: true,
                    vote: false,
                    decline_call: false,
                },
            },
            body: "Reply body".into(),
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            message_type: Some("text".into()),
            forward_transport: Some(TimelineForwardTransport::Text),
            media_filename: None,
            media_caption: None,
            edited: false,
            reply: Some(reply),
            thread_root: Some("$root:example.org".into()),
            thread: Some(thread),
            reactions: Vec::new(),
            media: None,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"eventId\":\"$parent:example.org\""));
        assert!(json.contains("\"rootEventId\":\"$root:example.org\""));
        assert!(json.contains("\"replyCount\":3"));
        assert!(json.contains("\"latestEventId\":\"$latest:example.org\""));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("access_token"));
        assert!(!json.contains("mxc://"));
    }

    #[test]
    fn sender_label_prefers_localpart_over_full_mxid() {
        assert_eq!(sender_localpart_or_id("@chris:matrix.whyland.com"), "chris");
        assert_eq!(sender_localpart_or_id("@spectre:example.org"), "spectre");
        assert_eq!(sender_localpart_or_id("not-an-mxid"), "not-an-mxid");
    }

    #[test]
    fn snapshot_and_delta_project_pinned_event_ids_without_secrets() {
        let snapshot = TimelineViewSnapshot {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            room_id: "!room:example.org".into(),
            revision: 0,
            position: TimelineViewPosition::LiveBottom,
            pagination: TimelinePaginationState {
                backward: TimelinePageState::Available,
                forward: TimelinePageState::Available,
            },
            read_state: TimelineReadState {
                own_read_event_id: None,
                unread_anchor_event_id: None,
                is_marked_unread: false,
            },
            pinned_event_ids: vec!["$pin:example.org".into(), "$pin2:example.org".into()],
            rows: Vec::new(),
            capabilities: TimelineViewCapabilities {
                mark_read: true,
                mark_unread: true,
                paginate_backward: true,
                paginate_forward: true,
            },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"pinnedEventIds\""));
        assert!(json.contains("$pin:example.org"));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("access_token"));

        let batch = TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            stream_id: "live:!room:example.org:1".into(),
            room_id: "!room:example.org".into(),
            revision: 1,
            ops: Vec::new(),
            read_state: None,
            pagination: None,
            pinned_event_ids: Some(vec!["$pin:example.org".into()]),
        };
        let batch_json = serde_json::to_string(&batch).unwrap();
        assert!(batch_json.contains("\"pinnedEventIds\""));
        assert!(batch_json.contains("$pin:example.org"));
    }
}
