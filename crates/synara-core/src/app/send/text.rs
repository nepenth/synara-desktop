//! Native composer text-send content builder.
//!
//! Live sends go through `RoomSendQueue`, not `Room::send`.

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

/// Shared UTF-8 byte budget for the complete outbound text representation.
///
/// Matrix sends the plain fallback and optional formatted representation in
/// one event, so the budget is deliberately applied to their combined bytes.
/// Presentation owners apply their separate, larger inbound rendering budget.
pub const MAX_OUTBOUND_TEXT_PAYLOAD_BYTES: usize = 65_536;

/// Raw mention entries are bounded before parsing or allocating the final set.
pub const MAX_OUTBOUND_MENTION_COUNT: usize = 256;

/// Matrix identifiers are capped at 255 UTF-8 bytes by the identifier grammar.
pub const MAX_MATRIX_IDENTIFIER_BYTES: usize = 255;

pub fn validate_outbound_text_payload(
    body: &str,
    formatted_body: Option<&str>,
) -> Result<(), &'static str> {
    let payload_bytes = body
        .len()
        .checked_add(formatted_body.map_or(0, str::len))
        .ok_or("d0.4-send-text-payload-too-large")?;
    if payload_bytes > MAX_OUTBOUND_TEXT_PAYLOAD_BYTES {
        return Err("d0.4-send-text-payload-too-large");
    }
    Ok(())
}

fn validate_matrix_identifier_length(
    value: &str,
    diagnostic_id: &'static str,
) -> Result<(), &'static str> {
    if value.len() > MAX_MATRIX_IDENTIFIER_BYTES {
        return Err(diagnostic_id);
    }
    Ok(())
}

pub fn parse_send_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    validate_matrix_identifier_length(room_id, "d0.4-send-invalid-room-id")?;
    room_id.parse().map_err(|_| "d0.4-send-invalid-room-id")
}

pub fn parse_reply_event_id(
    reply_to: Option<String>,
) -> Result<Option<OwnedEventId>, &'static str> {
    reply_to
        .map(|event_id| {
            validate_matrix_identifier_length(&event_id, "d0.4-send-invalid-reply-event-id")?;
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
            validate_matrix_identifier_length(&event_id, "v-send.5-invalid-thread-root-event-id")?;
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
            if txn_id.is_empty() {
                return Err("d0.4-send-invalid-transaction-id");
            }
            validate_matrix_identifier_length(&txn_id, "d0.4-send-invalid-transaction-id")?;
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
    validate_outbound_text_payload(&body, formatted_body.as_deref())?;
    let mut content = match (msg_type.as_deref().unwrap_or("m.text"), formatted_body) {
        ("m.text", Some(html)) => RoomMessageEventContent::text_html(body, html),
        ("m.text", None) => RoomMessageEventContent::text_plain(body),
        ("m.emote", Some(html)) => RoomMessageEventContent::emote_html(body, html),
        ("m.emote", None) => RoomMessageEventContent::emote_plain(body),
        ("m.notice", Some(html)) => RoomMessageEventContent::notice_html(body, html),
        ("m.notice", None) => RoomMessageEventContent::notice_plain(body),
        _ => return Err("v-send.4-invalid-message-type"),
    };
    let mentions = validated_mentions(mention_user_ids, mention_room)?;
    content.mentions = Some(mentions);
    content.relates_to = match (thread_root, reply_to) {
        (Some(root), Some(reply)) => Some(Relation::Thread(Thread::reply(root, reply))),
        (Some(root), None) => Some(Relation::Thread(Thread::without_fallback(root))),
        (None, Some(reply)) => Some(Relation::Reply(Reply::with_event_id(reply))),
        (None, None) => None,
    };
    Ok(content)
}

pub fn validated_mentions(
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
) -> Result<Mentions, &'static str> {
    let mention_user_ids = mention_user_ids.unwrap_or_default();
    if mention_user_ids.len() > MAX_OUTBOUND_MENTION_COUNT {
        return Err("v-send.4-too-many-mentions");
    }
    let mut user_ids = BTreeSet::new();
    for user_id in mention_user_ids {
        validate_matrix_identifier_length(&user_id, "v-send.4-mention-user-id-too-long")?;
        user_ids.insert(
            user_id
                .parse::<OwnedUserId>()
                .map_err(|_| "v-send.4-invalid-mention-user-id")?,
        );
    }
    let mut mentions = Mentions::new();
    mentions.user_ids = user_ids;
    mentions.room = mention_room;
    Ok(mentions)
}

pub async fn send_message_to_room(
    room: &Room,
    content: RoomMessageEventContent,
    _txn_id: Option<OwnedTransactionId>,
) -> Result<super::QueuedSendAck, &'static str> {
    // `RoomSendQueue::send` does not take a caller transaction id. The SDK
    // txn becomes the product local echo id after enqueue.
    super::send_event_via_room_queue(room, content.into())
        .await
        .map_err(|error| error.diagnostic_id)
}

pub(super) fn send_message_error_diagnostic(error: &matrix_sdk::Error) -> &'static str {
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

/// Whether Retry can help a send that failed with `cause`
/// ([`send_failure_cause`]). The homeserver refuses the same request again
/// when it is too large, forbidden, otherwise rejected (4xx), sent to a room
/// the user left, or has unusable media, so those rows offer Discard only.
/// Every other cause stays retryable, including the identity and
/// verification wedges the user resolves first, and network or server
/// trouble.
pub fn send_failure_is_retryable(cause: &str) -> bool {
    !matches!(
        cause,
        "too_large" | "forbidden" | "rejected" | "not_joined" | "media"
    )
}

/// Closed, privacy-safe cause of a failed send: `too_large`, `forbidden`,
/// `rate_limited`, `auth`, `network`, `server_error`, `rejected`,
/// `encryption`, `insecure_devices`, `identity_violation`,
/// `verification_required`, `media`, `not_joined` or `unknown`.
///
/// A live failure carries the SDK error itself. After a relaunch the SDK
/// only keeps the wedge's `GenericApiError` text, so its `[status` prefix
/// (ruma's `[413 / M_TOO_LARGE] …` display) is read instead; nothing else
/// from that text leaves Core.
pub fn send_failure_cause(error: &matrix_sdk::Error) -> &'static str {
    match error {
        matrix_sdk::Error::Http(error) => http_failure_cause(error),
        matrix_sdk::Error::SendQueueWedgeError(wedge) => wedge_failure_cause(wedge),
        matrix_sdk::Error::AuthenticationRequired => "auth",
        matrix_sdk::Error::WrongRoomState(_) => "not_joined",
        matrix_sdk::Error::ConcurrentRequestFailed => "network",
        matrix_sdk::Error::BadCryptoStoreState
        | matrix_sdk::Error::NoOlmMachine
        | matrix_sdk::Error::CryptoStoreError(_)
        | matrix_sdk::Error::OlmError(_)
        | matrix_sdk::Error::MegolmError(_) => "encryption",
        _ => "unknown",
    }
}

fn http_failure_cause(error: &matrix_sdk::HttpError) -> &'static str {
    use matrix_sdk::ruma::api::error::ErrorKind;
    use matrix_sdk::HttpError;

    match error {
        HttpError::Reqwest(_) => "network",
        HttpError::Cached(error) => http_failure_cause(error),
        HttpError::Api(_) => match error.client_api_error_kind() {
            Some(ErrorKind::TooLarge) => "too_large",
            Some(ErrorKind::Forbidden | ErrorKind::GuestAccessForbidden) => "forbidden",
            Some(ErrorKind::LimitExceeded(_)) => "rate_limited",
            Some(ErrorKind::MissingToken | ErrorKind::UnknownToken(_)) => "auth",
            _ => error.as_client_api_error().map_or("rejected", |api| {
                status_failure_cause(api.status_code.as_u16())
            }),
        },
        _ => "unknown",
    }
}

fn status_failure_cause(status: u16) -> &'static str {
    match status {
        413 => "too_large",
        403 => "forbidden",
        429 => "rate_limited",
        401 => "auth",
        500..=599 => "server_error",
        _ => "rejected",
    }
}

fn wedge_failure_cause(error: &matrix_sdk::QueueWedgeError) -> &'static str {
    use matrix_sdk::QueueWedgeError;

    match error {
        QueueWedgeError::InsecureDevices { .. } => "insecure_devices",
        QueueWedgeError::IdentityViolations { .. } => "identity_violation",
        QueueWedgeError::CrossVerificationRequired => "verification_required",
        QueueWedgeError::MissingMediaContent | QueueWedgeError::InvalidMimeType { .. } => "media",
        QueueWedgeError::GenericApiError { msg } => msg
            .find('[')
            .and_then(|start| msg.get(start + 1..start + 4))
            .and_then(|status| status.parse::<u16>().ok())
            .map_or("unknown", status_failure_cause),
    }
}

/// The wedged-send diagnostic for one cause. Every id keeps the
/// `d0.4-send-queue-wedged` prefix so callers that only need "wedged" can
/// match on it.
pub fn wedged_send_diagnostic(cause: &str) -> &'static str {
    match cause {
        "too_large" => "d0.4-send-queue-wedged-too-large",
        "forbidden" => "d0.4-send-queue-wedged-forbidden",
        "rejected" => "d0.4-send-queue-wedged-rejected",
        "encryption" => "d0.4-send-queue-wedged-encryption",
        "insecure_devices" => "d0.4-send-queue-wedged-insecure-devices",
        "identity_violation" => "d0.4-send-queue-wedged-identity-violation",
        "verification_required" => "d0.4-send-queue-wedged-verification-required",
        "media" => "d0.4-send-queue-wedged-media",
        "not_joined" => "d0.4-send-queue-wedged-not-joined",
        _ => WEDGED_SEND_DIAGNOSTIC,
    }
}

/// Generic wedged-send diagnostic, and the prefix of every cause-specific one.
pub const WEDGED_SEND_DIAGNOSTIC: &str = "d0.4-send-queue-wedged";

/// True for [`WEDGED_SEND_DIAGNOSTIC`] and every cause-specific variant.
pub fn is_wedged_send_diagnostic(diagnostic_id: &str) -> bool {
    diagnostic_id.starts_with(WEDGED_SEND_DIAGNOSTIC)
}

pub fn parse_edit_event_id(event_id: &str) -> Result<OwnedEventId, &'static str> {
    validate_matrix_identifier_length(event_id, "v-send.r-edit-invalid-event-id")?;
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
