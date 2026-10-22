//! Product composer sends go through `RoomSendQueue`.
//!
//! `send_text` returns once the SDK's persisted send queue holds the request
//! (`queued` plus its transaction id); the outcome follows on the timeline
//! row. A repeat with the caller's transaction id never creates a second
//! event. A wedged unrecoverable send blocks later sends in that room until
//! abort or unwedge. HTTP 520 is recoverable and must not wedge. Re-enabling
//! a room after a recoverable error belongs to the recovery owner, which
//! never wakes a paused client. `extra_content` stays unused.

use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::ruma::{event_id, mxc_uri, room_id, RoomId};
use matrix_sdk::send_queue::RoomSendQueueUpdate;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use serde_json::json;
use synara_core::app::send::{
    send_room_attachment, MatrixSendTextResult, OutboundTextMessage, SendRoomAttachmentRequest,
};
use synara_core::app::timeline::NativeTimelineOwner;
use synara_core::dto::{LocalEchoState, SendStatus};
use tokio::sync::broadcast;
use wiremock::ResponseTemplate;

#[derive(Debug, PartialEq, Eq)]
enum SendOutcome {
    Sent(String),
    Failed { recoverable: bool },
}

/// The SDK's next outcome for `txn` on a subscription taken before the send.
async fn next_outcome(
    watch: &mut broadcast::Receiver<RoomSendQueueUpdate>,
    txn: &str,
) -> SendOutcome {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match watch.recv().await {
                Ok(RoomSendQueueUpdate::SentEvent {
                    transaction_id,
                    event_id,
                }) if transaction_id.as_str() == txn => {
                    return SendOutcome::Sent(event_id.to_string())
                }
                Ok(RoomSendQueueUpdate::SendError {
                    transaction_id,
                    is_recoverable,
                    ..
                }) if transaction_id.as_str() == txn => {
                    return SendOutcome::Failed {
                        recoverable: is_recoverable,
                    }
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => panic!("send queue closed"),
            }
        }
    })
    .await
    .expect("send outcome timed out")
}

async fn send(
    owner: &NativeTimelineOwner,
    room_id: &RoomId,
    body: &str,
    txn_id: Option<&str>,
) -> Result<MatrixSendTextResult, &'static str> {
    owner
        .send_text(
            room_id.to_string(),
            body.to_owned(),
            None,
            None,
            None,
            None,
            None,
            None,
            txn_id.map(str::to_owned),
        )
        .await
}

async fn projected(
    owner: &NativeTimelineOwner,
    room_id: &RoomId,
    txn: &str,
) -> Option<OutboundTextMessage> {
    owner
        .outbound_text_for_room(room_id.as_str())
        .await
        .into_iter()
        .find(|item| item.local_txn_id == txn)
}

/// The follower records SDK outcomes after `send_text` returned.
async fn wait_for_projected_state(
    owner: &NativeTimelineOwner,
    room_id: &RoomId,
    txn: &str,
    state: LocalEchoState,
) -> OutboundTextMessage {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(item) = projected(owner, room_id, txn).await {
                if item.state == state {
                    return item;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("projected send never reached {state:?}"))
}

fn room_send_request_count(requests: &[wiremock::Request]) -> usize {
    requests
        .iter()
        .filter(|request| {
            request.method == wiremock::http::Method::PUT
                && request.url.path().contains("/send/m.room.message/")
        })
        .count()
}

async fn sent_requests(server: &MatrixMockServer) -> usize {
    room_send_request_count(
        &server
            .server()
            .received_requests()
            .await
            .unwrap_or_default(),
    )
}

fn transient_failure() -> ResponseTemplate {
    ResponseTemplate::new(500).set_body_json(json!({
        "errcode": "M_UNKNOWN",
        "error": "transient failure",
    }))
}

#[tokio::test]
async fn send_text_returns_once_queued_and_a_caller_txn_repeat_reports_sent() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-ok:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // Slow enough that `send_text` must answer before the homeserver does.
    server
        .mock_room_send()
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "event_id": "$queued-text" }))
                .set_delay(Duration::from_millis(400)),
        )
        .expect(1)
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 41);
    let first = send(&owner, room_id, "hello queue", Some("synara-client-1"))
        .await
        .expect("queued text send");
    assert_eq!(
        first.status, "queued",
        "Core answers once the SDK queue holds it"
    );
    assert!(first.event_id.is_empty());
    assert!(!first.local_txn_id.is_empty());
    assert_ne!(
        first.local_txn_id, "synara-client-1",
        "the SDK picks its own event transaction id"
    );
    let queued = projected(&owner, room_id, &first.local_txn_id)
        .await
        .expect("projected");
    assert_eq!(queued.state, LocalEchoState::Sending);
    assert_eq!(queued.client_txn_id.as_deref(), Some("synara-client-1"));

    assert_eq!(
        next_outcome(&mut watch, &first.local_txn_id).await,
        SendOutcome::Sent("$queued-text".into())
    );
    let sent =
        wait_for_projected_state(&owner, room_id, &first.local_txn_id, LocalEchoState::Sent).await;
    assert_eq!(sent.event_id.as_deref(), Some("$queued-text"));

    // A retry or second flush with the same caller txn after the send landed.
    let repeat = send(&owner, room_id, "hello queue", Some("synara-client-1"))
        .await
        .expect("repeat");
    assert_eq!(repeat.status, "sent");
    assert_eq!(repeat.event_id, "$queued-text");
    assert_eq!(repeat.local_txn_id, first.local_txn_id);
    assert_eq!(
        sent_requests(&server).await,
        1,
        "exactly one event reached the server"
    );
}

#[tokio::test]
async fn caller_txn_repeat_while_queued_never_creates_a_second_event() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-idempotent:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(transient_failure())
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 49);
    let first = send(&owner, room_id, "ok", Some("synara-client-2"))
        .await
        .expect("queued");
    assert_eq!(
        next_outcome(&mut watch, &first.local_txn_id).await,
        SendOutcome::Failed { recoverable: true }
    );

    for caller_txn in ["synara-client-2", first.local_txn_id.as_str()] {
        let repeat = send(&owner, room_id, "ok", Some(caller_txn))
            .await
            .expect("repeat while queued");
        assert_eq!(repeat.status, "queued");
        assert!(repeat.event_id.is_empty());
        assert_eq!(repeat.local_txn_id, first.local_txn_id);
    }
    let (echoes, _) = room.send_queue().subscribe().await.expect("echoes");
    assert_eq!(echoes.len(), 1, "a repeat must not enqueue a second event");

    // The same body under a new caller txn is a new message ("ok", "ok").
    let second = send(&owner, room_id, "ok", Some("synara-client-3"))
        .await
        .expect("second message");
    assert_ne!(second.local_txn_id, first.local_txn_id);
    let (echoes, _) = room.send_queue().subscribe().await.expect("echoes");
    assert_eq!(echoes.len(), 2);
}

#[tokio::test]
async fn recoverable_failure_leaves_reenabling_to_the_recovery_owner() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(transient_failure())
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 45);
    let first = send(&owner, room_id, "first (recoverable failure)", None)
        .await
        .expect("queued");
    assert_eq!(
        next_outcome(&mut watch, &first.local_txn_id).await,
        SendOutcome::Failed { recoverable: true }
    );
    let attempts = sent_requests(&server).await;
    assert_eq!(
        attempts, 3,
        "one logical attempt: the SDK's short HTTP retry"
    );

    // The send follower must not wake the room: an immediate re-enable
    // retried at once while offline and doubled the error traffic.
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(
        !room.send_queue().is_enabled(),
        "the room stays parked until the recovery owner re-enables it"
    );
    assert_eq!(sent_requests(&server).await, attempts);
    let parked = projected(&owner, room_id, &first.local_txn_id)
        .await
        .expect("projected");
    assert_eq!(
        parked.state,
        LocalEchoState::Sending,
        "still queued, not failed"
    );
    assert!(
        !synara_core::app::send::queued_send_is_wedged(&room, &first.local_txn_id)
            .await
            .unwrap_or(true),
        "recoverable failure must not mark the SDK request wedged"
    );
}

#[tokio::test]
async fn unrecoverable_send_wedges_with_its_cause_and_abort_unblocks_later_send() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-wedge:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .error_too_large()
        .mock_once()
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 42));
    let first = send(&owner, room_id, "too big", None)
        .await
        .expect("queued before the server answers");
    let wedged_txn = first.local_txn_id.clone();
    assert_eq!(
        next_outcome(&mut watch, &wedged_txn).await,
        SendOutcome::Failed { recoverable: false }
    );
    let wedged =
        wait_for_projected_state(&owner, room_id, &wedged_txn, LocalEchoState::Wedged).await;
    assert_eq!(
        wedged.failure_diagnostic_id,
        Some("d0.4-send-queue-wedged-too-large"),
        "a 413 must be distinguishable from a 403"
    );
    assert!(
        synara_core::app::send::queued_send_is_wedged(&room, &wedged_txn)
            .await
            .expect("inspect wedge"),
        "SDK local echo must carry a wedge error"
    );

    let second = send(&owner, room_id, "second", None)
        .await
        .expect("a later send still queues");
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(
        sent_requests(&server).await,
        1,
        "the wedge blocks later sends in the room"
    );
    server
        .mock_room_send()
        .ok(event_id!("$queued-second"))
        .mock_once()
        .mount()
        .await;
    owner
        .abort_send(room_id.as_str(), &wedged_txn)
        .await
        .expect("abort wedged send");
    let (echoes, _) = room
        .send_queue()
        .subscribe()
        .await
        .expect("local echoes after abort");
    assert!(
        echoes
            .iter()
            .all(|echo| echo.transaction_id.as_str() != wedged_txn),
        "abort must remove the transaction from the SDK local-echo list"
    );
    assert_eq!(
        next_outcome(&mut watch, &second.local_txn_id).await,
        SendOutcome::Sent("$queued-second".into())
    );
    wait_for_projected_state(&owner, room_id, &wedged_txn, LocalEchoState::Cancelled).await;
}

#[tokio::test]
async fn recoverable_poll_failure_is_reported_queued_not_failed() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-poll-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // Exhaust the SDK's short HTTP retry so the room queue sees one
    // recoverable `SendError` for this poll.
    server
        .mock_room_send()
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "errcode": "M_UNKNOWN",
            "error": "transient failure",
        })))
        .up_to_n_times(3)
        .expect(3)
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 47);
    let poll = owner
        .send_poll(
            room_id.to_string(),
            "Lunch?".into(),
            vec!["Yes".into(), "No".into()],
            1,
            None,
            None,
        )
        .await
        .expect("a recoverable poll failure is still queued, not a failed send");
    assert_eq!(
        poll.status, "queued",
        "reporting a still-queued poll as failed invites a duplicate poll"
    );
    assert!(poll.event_id.is_empty());

    // The SDK still holds exactly that request, recoverably.
    let (echoes, _) = room.send_queue().subscribe().await.expect("echoes");
    assert_eq!(echoes.len(), 1, "the poll stays in the persisted queue");
    let txn = echoes[0].transaction_id.to_string();
    assert!(!synara_core::app::send::queued_send_is_wedged(&room, &txn)
        .await
        .unwrap_or(true));
}

fn attachment_request(room_id: &str, transaction_id: &str) -> SendRoomAttachmentRequest {
    SendRoomAttachmentRequest {
        room_id: room_id.to_owned(),
        filename: "notes.txt".into(),
        mime_type: "text/plain".into(),
        payload: b"attachment bytes".to_vec(),
        caption: None,
        formatted_caption: None,
        reply_to: None,
        thread_root: None,
        transaction_id: Some(transaction_id.to_owned()),
        mention_user_ids: None,
        mention_room: false,
    }
}

#[tokio::test]
async fn sent_attachment_reports_event_and_caller_transaction_id() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-attachment-ok:example.org");
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // The send queue checks the upload size limit before uploading.
    server
        .mock_authenticated_media_config()
        .ok_default()
        .mount()
        .await;
    server
        .mock_upload()
        .ok(mxc_uri!("mxc://example.org/attachment"))
        .mock_once()
        .mount()
        .await;
    server
        .mock_room_send()
        .ok(event_id!("$sent-attachment"))
        .mock_once()
        .mount()
        .await;

    let result = send_room_attachment(
        &client,
        attachment_request(room_id.as_str(), "synara-attachment-sent"),
    )
    .await
    .expect("attachment sent");
    assert_eq!(result.status, SendStatus::Sent);
    assert_eq!(result.event_id, "$sent-attachment");
    assert_eq!(result.local_txn_id, "synara-attachment-sent");
}

#[tokio::test]
async fn recoverable_attachment_failure_is_queued_with_its_transaction_id() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-attachment-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // The send queue checks the upload size limit before uploading.
    server
        .mock_authenticated_media_config()
        .ok_default()
        .mount()
        .await;
    server
        .mock_upload()
        .ok(mxc_uri!("mxc://example.org/attachment"))
        .mount()
        .await;
    // Exhaust the SDK's short HTTP retry so the room queue sees one
    // recoverable `SendError` for the media event.
    server
        .mock_room_send()
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "errcode": "M_UNKNOWN",
            "error": "transient failure",
        })))
        .up_to_n_times(3)
        .expect(3)
        .mount()
        .await;

    let result = send_room_attachment(
        &client,
        attachment_request(room_id.as_str(), "synara-attachment-queued"),
    )
    .await
    .expect("a recoverable attachment failure is still queued, not failed");
    // iOS used to build a timeline row from `event_id` alone; a queued result
    // has none, so the shell needs the transaction id to name and later
    // replace its pending row.
    assert_eq!(result.status, SendStatus::Queued);
    assert!(result.event_id.is_empty());
    assert_eq!(result.local_txn_id, "synara-attachment-queued");

    // The SDK local echo the timeline shows carries that same transaction id.
    let (echoes, _) = room.send_queue().subscribe().await.expect("echoes");
    assert!(echoes
        .iter()
        .any(|echo| echo.transaction_id.as_str() == result.local_txn_id));
}

#[test]
fn queued_send_outcome_folds_only_still_queued_failures() {
    use synara_core::app::send::{
        queued_send_outcome, QueuedSendAck, QueuedSendError, QueuedSendOutcome,
    };
    let sent = queued_send_outcome(Ok(QueuedSendAck {
        event_id: "$sent".into(),
        transaction_id: "t1".into(),
    }))
    .expect("sent");
    assert_eq!(sent.status("voted"), "voted");
    assert_eq!(sent.event_id(), "$sent");

    let queued = queued_send_outcome(Err(QueuedSendError {
        diagnostic_id: "d0.4-send-queue-timeout",
        transaction_id: Some("t2".into()),
        wedged: false,
        cancelled: false,
        still_queued: true,
    }))
    .expect("still queued folds into an outcome");
    assert_eq!(
        queued,
        QueuedSendOutcome::Queued {
            transaction_id: "t2".into()
        }
    );
    assert_eq!(queued.status("sent"), "queued");
    assert!(queued.event_id().is_empty());
    assert_eq!(queued.transaction_id(), "t2");
    assert_eq!(sent.transaction_id(), "t1");

    for (wedged, cancelled) in [(true, false), (false, true), (false, false)] {
        assert!(queued_send_outcome(Err(QueuedSendError {
            diagnostic_id: "d0.4-send-queue-wedged",
            transaction_id: Some("t3".into()),
            wedged,
            cancelled,
            still_queued: false,
        }))
        .is_err());
    }
}

#[tokio::test]
async fn retry_unwedges_a_wedged_send_under_its_transaction_id() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-unwedge:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .error_too_large()
        .mock_once()
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 43);
    let txn = send(&owner, room_id, "retry me", None)
        .await
        .expect("queued")
        .local_txn_id;
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Failed { recoverable: false }
    );
    server
        .mock_room_send()
        .ok(event_id!("$unwedged"))
        .mock_once()
        .mount()
        .await;
    // `matrix_local_echo_retry` / `SharedCore::local_echo_retry` route here.
    owner
        .retry_send(room_id.as_str(), &txn)
        .await
        .expect("retry wedged echo");
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Sent("$unwedged".into())
    );
    let sent = wait_for_projected_state(&owner, room_id, &txn, LocalEchoState::Sent).await;
    assert_eq!(sent.event_id.as_deref(), Some("$unwedged"));
    assert_eq!(
        sent_requests(&server).await,
        2,
        "Retry resends the same request"
    );
}

#[tokio::test]
async fn retry_wakes_a_parked_recoverable_send_without_a_new_event() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-retry-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(transient_failure())
        .up_to_n_times(3)
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 50);
    let txn = send(&owner, room_id, "parked", None)
        .await
        .expect("queued")
        .local_txn_id;
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Failed { recoverable: true }
    );
    server
        .mock_room_send()
        .ok(event_id!("$retried"))
        .mock_once()
        .mount()
        .await;
    owner
        .retry_send(room_id.as_str(), &txn)
        .await
        .expect("retry parked echo");
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Sent("$retried".into())
    );
    let (echoes, _) = room.send_queue().subscribe().await.expect("echoes");
    assert!(
        echoes.is_empty(),
        "the one request was sent, nothing was added"
    );
}

#[tokio::test]
async fn http_520_does_not_wedge_the_send_queue() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-520:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(ResponseTemplate::new(520).set_body_json(json!({})))
        .up_to_n_times(3)
        .expect(3)
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 44);
    let txn = send(&owner, room_id, "cloudflare blip", None)
        .await
        .expect("queued")
        .local_txn_id;
    // `sent` after an HTTP-layer retry, or a recoverable error; never wedged.
    match next_outcome(&mut watch, &txn).await {
        SendOutcome::Sent(_) => {}
        SendOutcome::Failed { recoverable } => assert!(recoverable, "520 must not wedge"),
    }
    let projected = projected(&owner, room_id, &txn).await.expect("projected");
    assert_ne!(projected.state, LocalEchoState::Wedged);
    assert!(
        !synara_core::app::send::queued_send_is_wedged(&room, &txn)
            .await
            .unwrap_or(true),
        "520 must not mark the SDK request wedged"
    );
}

#[tokio::test]
async fn discarding_a_parked_recoverable_failure_never_sends_it() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-discard-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(transient_failure())
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 46);
    let txn = send(&owner, room_id, "discard me", None)
        .await
        .expect("queued")
        .local_txn_id;
    // The request is then parked in the store with the room queue disabled,
    // which is the state a restored unsent echo is in when the user presses
    // Discard.
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Failed { recoverable: true }
    );
    assert!(
        !room.send_queue().is_enabled(),
        "the SDK disables the room queue after a send error"
    );

    let before = sent_requests(&server).await;
    let aborted = owner
        .abort_send(room_id.as_str(), &txn)
        .await
        .expect("discard parked send");
    assert!(aborted, "a parked request is still local and must abort");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        sent_requests(&server).await,
        before,
        "discard must abort before re-enabling the queue, so the message is never sent"
    );
    let (echoes, _) = room
        .send_queue()
        .subscribe()
        .await
        .expect("local echoes after discard");
    assert!(
        echoes
            .iter()
            .all(|echo| echo.transaction_id.as_str() != txn),
        "discard must remove the transaction from the SDK local-echo list"
    );
    wait_for_projected_state(&owner, room_id, &txn, LocalEchoState::Cancelled).await;
    assert_eq!(
        send(&owner, room_id, "discard me", Some(&txn)).await,
        Err("d0.4-send-queue-cancelled"),
        "a discarded transaction is never sent again"
    );
}

#[tokio::test]
async fn discard_after_send_reports_not_aborted_and_keeps_the_sent_state() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-discard-sent:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .ok(event_id!("$already-sent"))
        .mock_once()
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 47);
    let txn = send(&owner, room_id, "too late to discard", None)
        .await
        .expect("queued")
        .local_txn_id;
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Sent("$already-sent".into())
    );
    wait_for_projected_state(&owner, room_id, &txn, LocalEchoState::Sent).await;
    // The SDK drops a sent request from its local-echo list, so discard of
    // that transaction cannot abort anything. The product queue must keep
    // the Sent state instead of recording a cancel.
    let result = owner.abort_send(room_id.as_str(), &txn).await;
    match result {
        Ok(aborted) => assert!(!aborted, "a sent request cannot be aborted"),
        Err(diagnostic) => assert_eq!(diagnostic, "d0.4-send-queue-handle-not-found"),
    }
    let projected = projected(&owner, room_id, &txn).await.expect("projected");
    assert_eq!(projected.state, LocalEchoState::Sent);
}

type RunningStream = std::pin::Pin<Box<dyn futures_util::Stream<Item = bool> + Send>>;

fn recovery_switch() -> (tokio::sync::mpsc::UnboundedSender<bool>, RunningStream) {
    let (running_tx, running_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();
    let running = Box::pin(futures_util::stream::unfold(
        running_rx,
        |mut rx| async move { rx.recv().await.map(|value| (value, rx)) },
    ));
    (running_tx, running)
}

#[tokio::test]
async fn sync_recovery_resends_a_parked_recoverable_failure() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-recovery:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // One logical attempt (3 HTTP tries) fails and parks the request.
    server
        .mock_room_send()
        .respond_with(transient_failure())
        .up_to_n_times(3)
        .mount()
        .await;

    let (running_tx, running) = recovery_switch();
    let recovery = synara_core::app::send::spawn_send_queue_recovery(client.clone(), running);
    running_tx.send(true).unwrap();

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 47);
    let txn = send(&owner, room_id, "send me after recovery", None)
        .await
        .expect("queued")
        .local_txn_id;
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Failed { recoverable: true }
    );
    assert!(!room.send_queue().is_enabled());

    // The server recovers and sync reports a recovery edge.
    server
        .mock_room_send()
        .ok(event_id!("$after-recovery"))
        .mount()
        .await;
    running_tx.send(false).unwrap();
    running_tx.send(true).unwrap();
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Sent("$after-recovery".into()),
        "sync recovery must re-enable the parked room queue"
    );
    recovery.abort();
}

#[tokio::test]
async fn recovery_never_reenables_the_send_queue_of_a_paused_client() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-recovery-paused:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(transient_failure())
        .mount()
        .await;

    let (running_tx, running) = recovery_switch();
    let recovery = synara_core::app::send::spawn_send_queue_recovery(client.clone(), running);
    running_tx.send(true).unwrap();

    // A recoverable failure arms the 5 s retry.
    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 51);
    let txn = send(&owner, room_id, "parked before pause", None)
        .await
        .expect("queued")
        .local_txn_id;
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Failed { recoverable: true }
    );

    // `stop_sync` pauses the client right after stopping sync. Here the pause
    // lands before the recovery owner sees sync stop: neither the armed retry
    // nor a later recovery edge may wake the queues on closed stores.
    client.pause().await.expect("pause client");
    assert!(!client.send_queue().is_enabled());
    running_tx.send(false).unwrap();
    running_tx.send(true).unwrap();
    tokio::time::sleep(synara_core::app::send::send_queue_retry_delay(1) + Duration::from_secs(1))
        .await;
    assert!(
        !client.send_queue().is_enabled(),
        "recovery must not re-enable the send queue of a paused client"
    );
    assert!(!room.send_queue().is_enabled());
    recovery.abort();
}

#[tokio::test]
async fn wedged_send_is_not_resent_by_sync_recovery() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-recovery-wedged:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .error_too_large()
        .mock_once()
        .mount()
        .await;

    let (running_tx, running) = recovery_switch();
    let recovery = synara_core::app::send::spawn_send_queue_recovery(client.clone(), running);
    running_tx.send(true).unwrap();
    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 48);
    let txn = send(&owner, room_id, "too big", None)
        .await
        .expect("queued")
        .local_txn_id;
    assert_eq!(
        next_outcome(&mut watch, &txn).await,
        SendOutcome::Failed { recoverable: false }
    );
    let before = sent_requests(&server).await;
    running_tx.send(false).unwrap();
    running_tx.send(true).unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        sent_requests(&server).await,
        before,
        "a wedged request waits for Retry or Discard"
    );
    assert!(synara_core::app::send::queued_send_is_wedged(&room, &txn)
        .await
        .unwrap());
    recovery.abort();
}
