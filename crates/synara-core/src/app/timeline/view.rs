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

use crate::dto::{EventId, RoomId, TimelineItemId, UserId};

use super::TimelineMediaRegistry;

pub const TIMELINE_VIEW_SCHEMA_VERSION: u32 = 1;
pub const NATIVE_TIMELINE_VIEW_UPDATED_EVENT: &str = "matrix-timeline-view-updated";

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEventRowBase {
    pub item_id: TimelineItemId,
    /// Absent only for a local echo which has not received a server event ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    pub sender_id: UserId,
    pub sender_name: String,
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
    let sender_id = event.sender().to_string();
    TimelineEventRowBase {
        item_id: item_id.to_owned(),
        event_id: event.event_id().map(ToString::to_string),
        sender_name: sender_id.clone(),
        sender_id,
        origin_server_ts: event.timestamp().get().into(),
        capabilities: project_row_action_capabilities(event),
    }
}

fn project_row_action_capabilities(event: &EventTimelineItem) -> TimelineRowCapabilities {
    let has_remote_id = event.event_id().is_some();
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
    let declineable =
        matches!(event.content(), TimelineItemContent::RtcNotification { .. }) && !event.is_own();
    TimelineRowCapabilities {
        // V-SEND.2 reaction toggle/ensure/redact is on the integration tip.
        react: has_remote_id && reactable,
        reply: has_remote_id && forwardable,
        edit: event.is_editable(),
        redact: has_remote_id,
        report: has_remote_id && !event.is_own(),
        pin: has_remote_id,
        forward: has_remote_id && forwardable,
        vote: has_remote_id && voteable,
        decline_call: has_remote_id && declineable,
    }
}

pub fn project_event_row(item_id: &str, event: &EventTimelineItem) -> TimelineViewRow {
    project_event_row_for_user(item_id, event, None, None)
}

fn project_event_row_for_user(
    item_id: &str,
    event: &EventTimelineItem,
    own_user_id: Option<&RumaUserId>,
    mut media_registry: Option<&mut TimelineMediaRegistry>,
) -> TimelineViewRow {
    let base = project_event_row_base(item_id, event);
    match event.content() {
        TimelineItemContent::MsgLike(content) => match &content.kind {
            MsgLikeKind::Message(message) => {
                let msgtype = message.msgtype();
                let (message_type, media) =
                    project_message_type_and_media(item_id, msgtype, media_registry.as_deref_mut());
                TimelineViewRow::Message(Box::new(TimelineMessageRow {
                    event: base,
                    body: message.body().to_owned(),
                    formatted_body: project_formatted_body(msgtype),
                    message_type,
                    edited: message.is_edited(),
                    reply: project_reply(content),
                    thread: project_thread_summary(content, event),
                    reactions: project_reactions(content, own_user_id),
                    media,
                }))
            }
            MsgLikeKind::Poll(poll) => {
                let results = poll.results();
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
                })
            }
            MsgLikeKind::Redacted => match base.event_id.clone() {
                Some(event_id) => TimelineViewRow::Redacted(TimelineRedactedRow {
                    item_id: base.item_id,
                    event_id,
                    summary: "Message removed".to_owned(),
                }),
                None => other_row(item_id, None, "Redacted local event"),
            },
            MsgLikeKind::UnableToDecrypt(_) => match base.event_id.clone() {
                Some(event_id) => {
                    TimelineViewRow::EncryptedUnavailable(TimelineEncryptedUnavailableRow {
                        item_id: base.item_id,
                        event_id,
                        reason_code: "unable_to_decrypt".to_owned(),
                    })
                }
                None => other_row(item_id, None, "Encrypted local event"),
            },
            MsgLikeKind::Sticker(sticker) => {
                let content = sticker.content();
                let media = media_registry.and_then(|registry| {
                    registry.register(
                        item_id,
                        content.source.clone().into(),
                        content.info.mimetype.clone(),
                        content
                            .info
                            .width
                            .and_then(|value| u32::try_from(u64::from(value)).ok()),
                        content
                            .info
                            .height
                            .and_then(|value| u32::try_from(u64::from(value)).ok()),
                        None,
                    )
                });
                match media {
                    Some(media) => TimelineViewRow::Sticker { event: base, media },
                    None => other_row(item_id, base.event_id, "Sticker unavailable"),
                }
            }
            _ => other_row(item_id, base.event_id, "Unsupported timeline event"),
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
            other_row(item_id, base.event_id, "Unsupported timeline event")
        }
    }
}

/// Project SDK-sanitized Matrix HTML when present and distinct from plain text.
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
    let TimelineDetails::Ready(event) = &details.event else {
        return None;
    };
    let body = event.content.as_message()?.body().to_owned();
    Some(TimelineReplyPreview {
        event_id: details.event_id.to_string(),
        sender_name: event.sender.to_string(),
        body,
    })
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
        return project_event_row_for_user(&item_id, event, own_user_id, None);
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
    media_registry: &mut TimelineMediaRegistry,
) -> TimelineViewRow {
    let item_id = item.unique_id().0.clone();
    if let Some(event) = item.as_event() {
        return project_event_row_for_user(&item_id, event, own_user_id, Some(media_registry));
    }
    project_timeline_item(item, own_user_id)
}

fn other_row(item_id: &str, event_id: Option<EventId>, summary: &str) -> TimelineViewRow {
    TimelineViewRow::Other(TimelineOtherRow {
        item_id: item_id.to_owned(),
        event_id,
        event_type: None,
        summary: summary.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMessageRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub body: String,
    /// Already-sanitized rendering markup; never raw event content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    pub edited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<TimelineReplyPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<TimelineThreadSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<TimelineReaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<TimelineMediaHandle>,
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
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEncryptedUnavailableRow {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineOtherRow {
    pub item_id: TimelineItemId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineViewRow {
    Message(Box<TimelineMessageRow>),
    Sticker {
        event: TimelineEventRowBase,
        media: TimelineMediaHandle,
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
    media_registry: &mut TimelineMediaRegistry,
) -> Vec<TimelineViewDeltaOp> {
    diffs
        .iter()
        .map(|diff| match diff {
            VectorDiff::Append { values } => TimelineViewDeltaOp::Append {
                rows: values
                    .iter()
                    .map(|item| project_timeline_item_with_media(item, own_user_id, media_registry))
                    .collect(),
            },
            VectorDiff::Clear => TimelineViewDeltaOp::Clear,
            VectorDiff::PushFront { value } => TimelineViewDeltaOp::PushFront {
                row: project_timeline_item_with_media(value, own_user_id, media_registry),
            },
            VectorDiff::PushBack { value } => TimelineViewDeltaOp::PushBack {
                row: project_timeline_item_with_media(value, own_user_id, media_registry),
            },
            VectorDiff::PopFront => TimelineViewDeltaOp::PopFront,
            VectorDiff::PopBack => TimelineViewDeltaOp::PopBack,
            VectorDiff::Insert { index, value } => TimelineViewDeltaOp::Insert {
                index: *index,
                row: project_timeline_item_with_media(value, own_user_id, media_registry),
            },
            VectorDiff::Set { index, value } => TimelineViewDeltaOp::Set {
                index: *index,
                row: project_timeline_item_with_media(value, own_user_id, media_registry),
            },
            VectorDiff::Remove { index } => TimelineViewDeltaOp::Remove { index: *index },
            VectorDiff::Truncate { length } => TimelineViewDeltaOp::Truncate { len: *length },
            VectorDiff::Reset { values } => TimelineViewDeltaOp::Reset {
                rows: values
                    .iter()
                    .map(|item| project_timeline_item_with_media(item, own_user_id, media_registry))
                    .collect(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::events::room::message::{
        EmoteMessageEventContent, NoticeMessageEventContent, TextMessageEventContent,
    };

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
    fn reply_and_thread_summary_serialize_product_shape_without_secrets() {
        let reply = TimelineReplyPreview {
            event_id: "$parent:example.org".into(),
            sender_name: "@alice:example.org".into(),
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
            message_type: Some("text".into()),
            edited: false,
            reply: Some(reply),
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
