//! Native composer text-send content builder.
//!
//! Live `Room::send` stays on the attached timeline owner.

use std::collections::BTreeSet;

use matrix_sdk::ruma::{
    events::{
        relation::{Reply, Thread},
        room::message::{Relation, RoomMessageEventContent},
        Mentions,
    },
    OwnedEventId, OwnedRoomId, OwnedTransactionId, OwnedUserId,
};
use matrix_sdk::Room;

pub fn parse_send_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    room_id.parse().map_err(|_| "d0.4-send-invalid-room-id")
}

pub fn parse_reply_event_id(
    reply_to: Option<String>,
) -> Result<Option<OwnedEventId>, &'static str> {
    reply_to
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| "d0.4-send-invalid-reply-event-id")
        })
        .transpose()
}

pub fn parse_thread_root_event_id(
    thread_root: Option<String>,
) -> Result<Option<OwnedEventId>, &'static str> {
    thread_root
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| "v-send.5-invalid-thread-root-event-id")
        })
        .transpose()
}

pub fn parse_transaction_id(
    txn_id: Option<String>,
) -> Result<Option<OwnedTransactionId>, &'static str> {
    txn_id
        .map(|txn_id| {
            if txn_id.is_empty() || txn_id.len() > 255 {
                return Err("d0.4-send-invalid-transaction-id");
            }
            Ok(OwnedTransactionId::from(txn_id))
        })
        .transpose()
}

pub fn message_content(
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> Result<RoomMessageEventContent, &'static str> {
    let mut content = match (msg_type.as_deref().unwrap_or("m.text"), formatted_body) {
        ("m.text", Some(html)) => RoomMessageEventContent::text_html(body, html),
        ("m.text", None) => RoomMessageEventContent::text_plain(body),
        ("m.emote", Some(html)) => RoomMessageEventContent::emote_html(body, html),
        ("m.emote", None) => RoomMessageEventContent::emote_plain(body),
        ("m.notice", Some(html)) => RoomMessageEventContent::notice_html(body, html),
        ("m.notice", None) => RoomMessageEventContent::notice_plain(body),
        _ => return Err("v-send.4-invalid-message-type"),
    };
    let user_ids = mention_user_ids
        .unwrap_or_default()
        .into_iter()
        .map(|user_id| {
            user_id
                .parse::<OwnedUserId>()
                .map_err(|_| "v-send.4-invalid-mention-user-id")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut mentions = Mentions::new();
    mentions.user_ids = user_ids;
    mentions.room = mention_room;
    content.mentions = Some(mentions);
    content.relates_to = match (thread_root, reply_to) {
        (Some(root), Some(reply)) => Some(Relation::Thread(Thread::reply(root, reply))),
        (Some(root), None) => Some(Relation::Thread(Thread::without_fallback(root))),
        (None, Some(reply)) => Some(Relation::Reply(Reply::with_event_id(reply))),
        (None, None) => None,
    };
    Ok(content)
}

pub async fn send_message_to_room(
    room: &Room,
    content: RoomMessageEventContent,
    txn_id: Option<OwnedTransactionId>,
) -> Result<String, &'static str> {
    let send = room.send(content);
    let result = match txn_id {
        Some(txn_id) => send.with_transaction_id(txn_id).await,
        None => send.await,
    };
    result
        .map(|result| result.response.event_id.to_string())
        .map_err(|_| "d0.4-send-sdk-failed")
}
