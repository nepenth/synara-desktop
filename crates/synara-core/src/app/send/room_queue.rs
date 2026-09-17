//! Product send owner: `RoomSendQueue` / `SendHandle`.
//!
//! Composer text, attachments, polls, edits, and ensure-reactions enqueue here
//! so 0.19 per-room order and wedging apply. `AttachmentConfig.extra_content`
//! is intentionally unused. HTTP 520 stays a transient HTTP-layer retry and
//! must not mark a request wedged.

use std::time::Duration;

use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::ruma::events::AnyMessageLikeEventContent;
use matrix_sdk::ruma::OwnedTransactionId;
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
}

impl QueuedSendError {
    fn failed(diagnostic_id: &'static str, transaction_id: Option<String>) -> Self {
        Self {
            diagnostic_id,
            transaction_id,
            wedged: false,
            cancelled: false,
        }
    }

    fn wedged(transaction_id: Option<String>) -> Self {
        Self {
            diagnostic_id: "d0.4-send-queue-wedged",
            transaction_id,
            wedged: true,
            cancelled: false,
        }
    }

    fn cancelled(transaction_id: Option<String>) -> Self {
        Self {
            diagnostic_id: "d0.4-send-queue-cancelled",
            transaction_id,
            wedged: false,
            cancelled: true,
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
                        return Err(QueuedSendError::failed(
                            send_message_error_diagnostic(error.as_ref()),
                            Some(txn.to_owned()),
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
        Err(_) => Err(QueuedSendError::failed(
            "d0.4-send-queue-timeout",
            Some(txn.to_owned()),
        )),
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

/// Abort one wedged (or still-local) request so later items can send.
pub async fn abort_queued_send(room: &Room, transaction_id: &str) -> Result<bool, QueuedSendError> {
    let handle = handle_for_transaction(room, transaction_id).await?;
    room.send_queue().set_enabled(true);
    handle.abort().await.map_err(|_| {
        QueuedSendError::failed(
            "d0.4-send-queue-abort-failed",
            Some(transaction_id.to_owned()),
        )
    })
}

/// Test/debug: inspect whether a local echo currently carries a wedge error.
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
        assert!(source.contains("d0.4-send-extra-content-forbidden"));
        assert!(source.contains("config.extra_content.is_some()"));
        assert!(source.contains("room.send_queue()"));
        assert!(!source.contains(&format!("{}{}", "room.", "send(")));
        assert!(!source.contains(&format!("{}{}", "room.", "send_attachment(")));
    }
}
