//! Product send owner: `RoomSendQueue` / `SendHandle`.
//!
//! Composer text, attachments, polls, edits, and ensure-reactions enqueue here
//! so 0.19 per-room order and wedging apply. `AttachmentConfig.extra_content`
//! is intentionally unused. HTTP 520 stays a transient HTTP-layer retry and
//! must not mark a request wedged.

use std::time::Duration;

use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::ruma::events::AnyMessageLikeEventContent;
use matrix_sdk::ruma::{EventId, OwnedTransactionId};
use matrix_sdk::send_queue::{
    LocalEchoContent, RoomSendQueueError, RoomSendQueueUpdate, SendHandle,
};
use matrix_sdk::Room;
use mime::Mime;
use tokio::sync::broadcast;
use tokio::time::timeout;

use super::text::send_message_error_diagnostic;

/// How long IPC waits for `SentEvent` / `SendError` / cancel for one request.
const QUEUED_SEND_WAIT: Duration = Duration::from_secs(30);

/// Successful `RoomSendQueue` send that has a server event id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedSendAck {
    pub event_id: String,
    pub transaction_id: String,
}

/// Privacy-safe send-queue failure. `transaction_id` is set once the SDK
/// local echo is known, even when the request later wedges or is cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedSendError {
    pub diagnostic_id: &'static str,
    pub transaction_id: Option<String>,
    pub wedged: bool,
    pub cancelled: bool,
    /// The request is still in the SDK's persisted queue and will be retried
    /// (recoverable error, or the wait deadline passed). The caller must not
    /// present this as a failure that invites a second send.
    pub still_queued: bool,
}

impl QueuedSendError {
    fn failed(diagnostic_id: &'static str, transaction_id: Option<String>) -> Self {
        Self {
            diagnostic_id,
            transaction_id,
            wedged: false,
            cancelled: false,
            still_queued: false,
        }
    }

    fn still_queued(diagnostic_id: &'static str, transaction_id: String) -> Self {
        Self {
            diagnostic_id,
            transaction_id: Some(transaction_id),
            wedged: false,
            cancelled: false,
            still_queued: true,
        }
    }

    fn wedged(transaction_id: Option<String>) -> Self {
        Self {
            diagnostic_id: "d0.4-send-queue-wedged",
            transaction_id,
            wedged: true,
            cancelled: false,
            still_queued: false,
        }
    }

    fn cancelled(transaction_id: Option<String>) -> Self {
        Self {
            diagnostic_id: "d0.4-send-queue-cancelled",
            transaction_id,
            wedged: false,
            cancelled: true,
            still_queued: false,
        }
    }
}

/// In-flight `RoomSendQueue` request waiting for `SentEvent`.
pub struct QueuedSendSession {
    updates: broadcast::Receiver<RoomSendQueueUpdate>,
    room: Room,
    pub transaction_id: String,
}

fn map_queue_error(error: RoomSendQueueError) -> QueuedSendError {
    let diagnostic_id = match error {
        RoomSendQueueError::RoomNotJoined => "d0.4-send-sdk-wrong-room-state",
        RoomSendQueueError::RoomDisappeared => "d0.4-send-sdk-insufficient-data",
        RoomSendQueueError::FailedToCreateAttachment => "v-send.1-attachment-sdk-failed",
        RoomSendQueueError::StorageError(_) => "d0.4-send-sdk-state-store-failed",
    };
    QueuedSendError::failed(diagnostic_id, None)
}

fn echo_matches_handle(content: &LocalEchoContent, handle: &SendHandle) -> bool {
    match content {
        LocalEchoContent::Event { send_handle, .. } => send_handle.created_at == handle.created_at,
        _ => false,
    }
}

async fn identify_transaction(
    queue: &matrix_sdk::send_queue::RoomSendQueue,
    updates: &mut broadcast::Receiver<RoomSendQueueUpdate>,
    handle: &SendHandle,
) -> Result<OwnedTransactionId, QueuedSendError> {
    loop {
        match updates.try_recv() {
            Ok(RoomSendQueueUpdate::NewLocalEvent(echo))
                if echo_matches_handle(&echo.content, handle) =>
            {
                return Ok(echo.transaction_id);
            }
            Ok(_) => continue,
            Err(broadcast::error::TryRecvError::Empty) => break,
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                if let Some(found) = transaction_from_local_echoes(queue, handle).await {
                    return Ok(found);
                }
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                return Err(QueuedSendError::failed("d0.4-send-queue-closed", None));
            }
        }
    }
    if let Some(found) = transaction_from_local_echoes(queue, handle).await {
        return Ok(found);
    }
    match timeout(QUEUED_SEND_WAIT, async {
        loop {
            match updates.recv().await {
                Ok(RoomSendQueueUpdate::NewLocalEvent(echo))
                    if echo_matches_handle(&echo.content, handle) =>
                {
                    return Ok(echo.transaction_id);
                }
                Ok(_) => {
                    if let Some(found) = transaction_from_local_echoes(queue, handle).await {
                        return Ok(found);
                    }
                }
                Err(_) => {
                    return Err(QueuedSendError::failed("d0.4-send-queue-closed", None));
                }
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(QueuedSendError::failed("d0.4-send-queue-timeout", None)),
    }
}

async fn transaction_from_local_echoes(
    queue: &matrix_sdk::send_queue::RoomSendQueue,
    handle: &SendHandle,
) -> Option<OwnedTransactionId> {
    let (echoes, _) = queue.subscribe().await.ok()?;
    echoes
        .into_iter()
        .find_map(|echo| echo_matches_handle(&echo.content, handle).then_some(echo.transaction_id))
}

/// Enqueue a message-like event on `RoomSendQueue` and return a session to
/// wait on. Vendor extra content keys are not merged.
pub async fn enqueue_event_via_room_queue(
    room: &Room,
    content: AnyMessageLikeEventContent,
) -> Result<QueuedSendSession, QueuedSendError> {
    let queue = room.send_queue();
    let (_echoes, mut updates) = queue
        .subscribe()
        .await
        .map_err(|_| QueuedSendError::failed("d0.4-send-queue-subscribe-failed", None))?;
    let handle = queue.send(content).await.map_err(map_queue_error)?;
    let transaction_id = identify_transaction(&queue, &mut updates, &handle).await?;
    Ok(QueuedSendSession {
        updates,
        room: room.clone(),
        transaction_id: transaction_id.to_string(),
    })
}

/// Enqueue an attachment on `RoomSendQueue` using the existing
/// `AttachmentConfig` (caption, mentions, reply, txn). Never sets
/// `extra_content`.
pub async fn enqueue_attachment_via_room_queue(
    room: &Room,
    filename: &str,
    mime_type: Mime,
    payload: Vec<u8>,
    config: AttachmentConfig,
) -> Result<QueuedSendSession, QueuedSendError> {
    if config.extra_content.is_some() {
        return Err(QueuedSendError::failed(
            "d0.4-send-extra-content-forbidden",
            None,
        ));
    }
    let queue = room.send_queue();
    let (_echoes, mut updates) = queue
        .subscribe()
        .await
        .map_err(|_| QueuedSendError::failed("d0.4-send-queue-subscribe-failed", None))?;
    let handle = queue
        .send_attachment(filename, mime_type, payload, config)
        .await
        .map_err(map_queue_error)?;
    let transaction_id = identify_transaction(&queue, &mut updates, &handle).await?;
    Ok(QueuedSendSession {
        updates,
        room: room.clone(),
        transaction_id: transaction_id.to_string(),
    })
}

/// Wait until this request is sent, cancelled, or fails (wedged or not).
pub async fn wait_for_queued_send(
    session: &mut QueuedSendSession,
) -> Result<QueuedSendAck, QueuedSendError> {
    let txn = session.transaction_id.as_str();
    let deadline = timeout(QUEUED_SEND_WAIT, async {
        loop {
            match session.updates.recv().await {
                Ok(RoomSendQueueUpdate::SentEvent {
                    transaction_id,
                    event_id,
                }) if transaction_id.as_str() == txn => {
                    return Ok(QueuedSendAck {
                        event_id: event_id.to_string(),
                        transaction_id: txn.to_owned(),
                    });
                }
                Ok(RoomSendQueueUpdate::SendError {
                    transaction_id,
                    error,
                    is_recoverable,
                }) if transaction_id.as_str() == txn => {
                    if is_recoverable {
                        // The SDK disables this room's local queue after
                        // *any* send error, recoverable or not (0.19
                        // `send_queue::mod.rs` `sending_task`). Only an
                        // unrecoverable (wedged) failure should keep later
                        // sends in this room blocked; a recoverable failure
                        // must not silently strand every following send
                        // behind a queue nobody re-enables. The failed
                        // request stays queued (not removed), so re-enabling
                        // here also lets the SDK retry it in order.
                        session.room.send_queue().set_enabled(true);
                        return Err(QueuedSendError::still_queued(
                            send_message_error_diagnostic(error.as_ref()),
                            txn.to_owned(),
                        ));
                    }
                    return Err(QueuedSendError::wedged(Some(txn.to_owned())));
                }
                Ok(RoomSendQueueUpdate::CancelledLocalEvent { transaction_id })
                    if transaction_id.as_str() == txn =>
                {
                    return Err(QueuedSendError::cancelled(Some(txn.to_owned())));
                }
                Ok(_) => continue,
                Err(_) => {
                    return Err(QueuedSendError::failed(
                        "d0.4-send-queue-closed",
                        Some(txn.to_owned()),
                    ));
                }
            }
        }
    })
    .await;
    match deadline {
        Ok(result) => result,
        // The request is still persisted and the SDK keeps sending it.
        Err(_) => Err(QueuedSendError::still_queued(
            "d0.4-send-queue-timeout",
            txn.to_owned(),
        )),
    }
}

/// How a caller should present one queued request after waiting on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuedSendOutcome {
    /// The homeserver accepted it.
    Sent(QueuedSendAck),
    /// Still in the SDK's persisted queue (recoverable error or the wait
    /// deadline): the SDK keeps retrying and the timeline row shows Sending
    /// with Discard. Reporting this as a failure would invite a duplicate.
    Queued { transaction_id: String },
}

impl QueuedSendOutcome {
    /// The wire `status` for a write that is either sent or still queued.
    pub fn status(&self, sent: &'static str) -> &'static str {
        match self {
            Self::Sent(_) => sent,
            Self::Queued { .. } => "queued",
        }
    }

    /// The typed send status for a write whose success value is `sent`.
    pub fn send_status(&self) -> crate::dto::SendStatus {
        match self {
            Self::Sent(_) => crate::dto::SendStatus::Sent,
            Self::Queued { .. } => crate::dto::SendStatus::Queued,
        }
    }

    /// The server event id, or empty while the request is still queued.
    pub fn event_id(&self) -> String {
        match self {
            Self::Sent(ack) => ack.event_id.clone(),
            Self::Queued { .. } => String::new(),
        }
    }
}

/// Fold a still-queued wait result into [`QueuedSendOutcome::Queued`];
/// every other failure stays an error.
pub fn queued_send_outcome(
    result: Result<QueuedSendAck, QueuedSendError>,
) -> Result<QueuedSendOutcome, QueuedSendError> {
    match result {
        Ok(ack) => Ok(QueuedSendOutcome::Sent(ack)),
        Err(QueuedSendError {
            still_queued: true,
            transaction_id: Some(transaction_id),
            ..
        }) => Ok(QueuedSendOutcome::Queued { transaction_id }),
        Err(error) => Err(error),
    }
}

/// Enqueue and wait. Used by poll / edit / ensure / forward where the product
/// harness does not need a `SendQueue` projection.
pub async fn send_event_via_room_queue(
    room: &Room,
    content: AnyMessageLikeEventContent,
) -> Result<QueuedSendAck, QueuedSendError> {
    let mut session = enqueue_event_via_room_queue(room, content).await?;
    wait_for_queued_send(&mut session).await
}

/// Enqueue an attachment and wait for the media event id.
pub async fn send_attachment_via_room_queue(
    room: &Room,
    filename: &str,
    mime_type: Mime,
    payload: Vec<u8>,
    config: AttachmentConfig,
) -> Result<QueuedSendAck, QueuedSendError> {
    let mut session =
        enqueue_attachment_via_room_queue(room, filename, mime_type, payload, config).await?;
    wait_for_queued_send(&mut session).await
}

async fn handle_for_transaction(
    room: &Room,
    transaction_id: &str,
) -> Result<SendHandle, QueuedSendError> {
    let (echoes, _) = room
        .send_queue()
        .subscribe()
        .await
        .map_err(|_| QueuedSendError::failed("d0.4-send-queue-subscribe-failed", None))?;
    for echo in echoes {
        if echo.transaction_id.as_str() != transaction_id {
            continue;
        }
        if let LocalEchoContent::Event { send_handle, .. } = echo.content {
            return Ok(send_handle);
        }
    }
    Err(QueuedSendError::failed(
        "d0.4-send-queue-handle-not-found",
        Some(transaction_id.to_owned()),
    ))
}

/// Re-enable the room queue and unwedge one request so later items can send.
pub async fn unwedge_queued_send(room: &Room, transaction_id: &str) -> Result<(), QueuedSendError> {
    let handle = handle_for_transaction(room, transaction_id).await?;
    room.send_queue().set_enabled(true);
    handle.unwedge().await.map_err(|_| {
        QueuedSendError::failed(
            "d0.4-send-queue-unwedge-failed",
            Some(transaction_id.to_owned()),
        )
    })
}

/// Re-enable one room queue so a recoverable failure that is still queued
/// retries under the same transaction id.
///
/// This is the `RoomSendQueue::set_enabled(true)` path that
/// [`wait_for_queued_send`] uses after a recoverable send failure. It does
/// not allocate a transaction id. The caller has already routed wedged
/// requests to [`unwedge_queued_send`]; this only confirms the echo is still
/// queued so a retry of a vanished request fails closed.
pub async fn reenable_queued_send(
    room: &Room,
    transaction_id: &str,
) -> Result<(), QueuedSendError> {
    let _handle = handle_for_transaction(room, transaction_id).await?;
    room.send_queue().set_enabled(true);
    Ok(())
}

/// Abort one unsent request, then let later items in the room send.
///
/// The abort runs before the room queue is re-enabled. After a send error
/// the SDK leaves the room queue disabled; re-enabling first would wake the
/// sending task, which could pick this very request up and send the message
/// the user just discarded. `Ok(false)` means the SDK no longer held the
/// request (it was already sent); the queue is left as it was.
pub async fn abort_queued_send(room: &Room, transaction_id: &str) -> Result<bool, QueuedSendError> {
    let handle = handle_for_transaction(room, transaction_id).await?;
    let aborted = handle.abort().await.map_err(|_| {
        QueuedSendError::failed(
            "d0.4-send-queue-abort-failed",
            Some(transaction_id.to_owned()),
        )
    })?;
    if aborted {
        room.send_queue().set_enabled(true);
    }
    Ok(aborted)
}

/// True when this echo carries an SDK `QueueWedgeError`.
///
/// The send queue sets `send_error` only for an unrecoverable failure
/// (`mark_as_wedged`). A recoverable failure stays queued with `send_error:
/// None`, so retry must call [`reenable_queued_send`] instead of unwedge.
pub async fn queued_send_is_wedged(
    room: &Room,
    transaction_id: &str,
) -> Result<bool, QueuedSendError> {
    let (echoes, _) = room
        .send_queue()
        .subscribe()
        .await
        .map_err(|_| QueuedSendError::failed("d0.4-send-queue-subscribe-failed", None))?;
    for echo in echoes {
        if echo.transaction_id.as_str() != transaction_id {
            continue;
        }
        return Ok(matches!(
            echo.content,
            LocalEchoContent::Event {
                send_error: Some(_),
                ..
            }
        ));
    }
    Ok(false)
}

/// Queue a redaction on `RoomSendQueue` and wait for it like any other send.
///
/// The queue orders the redaction after earlier queued edits or reactions on
/// the same event and keeps it across offline periods and restarts, which a
/// direct `Room::redact` request does not.
pub async fn redact_via_room_queue(
    room: &Room,
    redacts: &EventId,
    reason: Option<&str>,
) -> Result<QueuedSendAck, QueuedSendError> {
    let queue = room.send_queue();
    let (_echoes, mut updates) = queue
        .subscribe()
        .await
        .map_err(|_| QueuedSendError::failed("d0.4-send-queue-subscribe-failed", None))?;
    queue
        .redact(redacts.to_owned(), reason)
        .await
        .map_err(map_queue_error)?;
    // `RoomSendQueue::redact` announces its local echo before returning, so the
    // transaction id is already buffered on this subscription.
    let transaction_id = loop {
        match updates.try_recv() {
            Ok(RoomSendQueueUpdate::NewLocalEvent(echo))
                if matches!(
                    &echo.content,
                    LocalEchoContent::Redaction { redacts: queued, .. } if **queued == *redacts
                ) =>
            {
                break echo.transaction_id;
            }
            Ok(_) | Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
            Err(_) => {
                return Err(QueuedSendError::failed(
                    "d0.4-send-queue-redaction-unidentified",
                    None,
                ))
            }
        }
    };
    let mut session = QueuedSendSession {
        updates,
        room: room.clone(),
        transaction_id: transaction_id.to_string(),
    };
    wait_for_queued_send(&mut session).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn extra_content_stays_off_the_product_send_owner() {
        let source = include_str!("room_queue.rs");
        assert!(source.contains("room.send_queue()"));
        assert!(source.contains("SendHandle"));
        assert!(source.contains("wait_for_queued_send"));
        assert!(source.contains("unwedge"));
        assert!(source.contains("abort"));
        assert!(source.contains("reenable_queued_send"));
        assert!(source.contains("set_enabled(true)"));
        assert!(source.contains("d0.4-send-extra-content-forbidden"));
        assert!(source.contains("config.extra_content.is_some()"));
        assert!(source.contains("room.send_queue()"));
        assert!(!source.contains(&format!("{}{}", "room.", "send(")));
        assert!(!source.contains(&format!("{}{}", "room.", "send_attachment(")));
    }

    #[test]
    fn discard_aborts_before_waking_the_room_queue() {
        // Re-enabling first wakes the SDK sending task, which can pick up the
        // parked request and send the message the user discarded. The mock
        // suite cannot force that race on a single-threaded runtime, so pin
        // the order here.
        let source = include_str!("room_queue.rs");
        let start = source
            .find("pub async fn abort_queued_send")
            .expect("abort_queued_send");
        let end = start
            + source[start..]
                .find("\n}\n")
                .expect("abort_queued_send body end");
        let body = &source[start..end];
        let abort = body.find("handle.abort()").expect("abort call");
        let enable = body
            .find(concat!("set_enabled", "(true)"))
            .expect("re-enable after abort");
        assert!(
            abort < enable,
            "abort must run before the queue is re-enabled"
        );
        assert!(body.contains("if aborted {"));
    }
}
