//! Native composer text-send content builder.
//!
//! Live `Room::send` stays on the attached timeline owner.

use std::collections::BTreeSet;

use matrix_sdk::ruma::{
    events::{
        relation::{Reply, Thread},
        room::message::{Relation, ReplacementMetadata, RoomMessageEventContent},
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
        .map_err(|error| send_message_error_diagnostic(&error))
}

fn send_message_error_diagnostic(error: &matrix_sdk::Error) -> &'static str {
    match error {
        matrix_sdk::Error::Http(error) => send_http_error_diagnostic(error),
        matrix_sdk::Error::AuthenticationRequired => "d0.4-send-sdk-auth-required",
        matrix_sdk::Error::InsufficientData => "d0.4-send-sdk-insufficient-data",
        matrix_sdk::Error::BadCryptoStoreState => "d0.4-send-sdk-crypto-store-state",
        matrix_sdk::Error::NoOlmMachine => "d0.4-send-sdk-no-olm-machine",
        matrix_sdk::Error::CryptoStoreError(_) => "d0.4-send-sdk-crypto-store-failed",
        matrix_sdk::Error::OlmError(_) => "d0.4-send-sdk-olm-failed",
        matrix_sdk::Error::MegolmError(_) => "d0.4-send-sdk-megolm-failed",
        matrix_sdk::Error::StateStore(_) => "d0.4-send-sdk-state-store-failed",
        matrix_sdk::Error::WrongRoomState(_) => "d0.4-send-sdk-wrong-room-state",
        matrix_sdk::Error::ConcurrentRequestFailed => "d0.4-send-sdk-concurrent-request-failed",
        _ => "d0.4-send-sdk-failed",
    }
}

fn send_http_error_diagnostic(error: &matrix_sdk::HttpError) -> &'static str {
    use matrix_sdk::ruma::api::error::ErrorKind;
    use matrix_sdk::HttpError;

    match error {
        HttpError::Reqwest(_) => "d0.4-send-sdk-http-network-failed",
        HttpError::IntoHttp(_) => "d0.4-send-sdk-http-request-failed",
        HttpError::RefreshToken(_) => "d0.4-send-sdk-http-refresh-failed",
        HttpError::Cached(error) => send_http_error_diagnostic(error),
        HttpError::Api(_) => match error.client_api_error_kind() {
            Some(ErrorKind::Forbidden | ErrorKind::GuestAccessForbidden) => {
                "d0.4-send-sdk-http-forbidden"
            }
            Some(ErrorKind::MissingToken | ErrorKind::UnknownToken(_)) => {
                "d0.4-send-sdk-http-auth-failed"
            }
            Some(ErrorKind::LimitExceeded(_)) => "d0.4-send-sdk-http-rate-limited",
            Some(
                ErrorKind::BadJson
                | ErrorKind::InvalidParam
                | ErrorKind::MissingParam
                | ErrorKind::NotJson,
            ) => "d0.4-send-sdk-http-invalid-request",
            Some(ErrorKind::NotFound) => "d0.4-send-sdk-http-not-found",
            Some(_) | None => "d0.4-send-sdk-http-api-failed",
        },
    }
}

pub fn parse_edit_event_id(event_id: &str) -> Result<OwnedEventId, &'static str> {
    event_id
        .parse()
        .map_err(|_| "v-send.r-edit-invalid-event-id")
}

pub fn edit_message_content(
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
    event_id: OwnedEventId,
) -> Result<RoomMessageEventContent, &'static str> {
    let content = message_content(
        body,
        msg_type,
        formatted_body,
        mention_user_ids,
        mention_room,
        None,
        None,
    )?;
    let mentions = content.mentions.clone();
    Ok(content.make_replacement(ReplacementMetadata::new(event_id, mentions)))
}
