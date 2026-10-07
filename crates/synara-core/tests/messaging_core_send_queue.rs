//! Product composer sends go through `RoomSendQueue`.
//!
//! A wedged unrecoverable send blocks later sends in that room until abort or
//! unwedge. HTTP 520 is recoverable and must not wedge. `extra_content` stays
//! unused. The live Synapse proof is the happy-path `RoomSendQueue` owner;
//! this mock-server test covers wedge / abort / unwedge.

use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::ruma::{event_id, room_id};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use serde_json::json;
use synara_core::app::timeline::NativeTimelineOwner;
use synara_core::dto::LocalEchoState;
use wiremock::ResponseTemplate;

#[tokio::test]
async fn send_text_via_room_send_queue_returns_event_id() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-ok:example.org");
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .ok(event_id!("$queued-text"))
        .mock_once()
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 41);
    let result = owner
        .send_text(
            room_id.to_string(),
            "hello queue".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("queued text send");
    assert_eq!(result.event_id, "$queued-text");
    assert_eq!(result.status, "sent");
    assert!(!result.local_txn_id.is_empty());
    let projected = owner.outbound_text_for_room(room_id.as_str()).await;
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].state, LocalEchoState::Sent);
    assert_eq!(projected[0].local_txn_id, result.local_txn_id);
}

#[tokio::test]
async fn unrecoverable_send_wedges_and_abort_unblocks_later_send() {
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

    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 42));
    let first = owner
        .send_text(
            room_id.to_string(),
            "too big".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("unrecoverable send must fail closed");
    assert_eq!(first, "d0.4-send-queue-wedged");
    let projected = owner.outbound_text_for_room(room_id.as_str()).await;
    assert_eq!(projected[0].state, LocalEchoState::Wedged);
    let wedged_txn = projected[0].local_txn_id.clone();
    assert!(
        synara_core::app::send::queued_send_is_wedged(&room, &wedged_txn)
            .await
            .expect("inspect wedge"),
        "SDK local echo must carry a wedge error"
    );

    let second = {
        let owner = Arc::clone(&owner);
        let room_id = room_id.to_string();
        tokio::spawn(async move {
            owner
                .send_text(
                    room_id,
                    "second".into(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(80)).await;
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
    let second = tokio::time::timeout(Duration::from_secs(8), second)
        .await
        .expect("second send must not hang after abort")
        .expect("join second send")
        .expect("second send after abort");
    assert_eq!(second.event_id, "$queued-second");
    assert_eq!(second.status, "sent");
}

#[tokio::test]
async fn recoverable_failure_does_not_strand_the_next_send() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // Exactly exhaust the SDK's internal `short_retry` (3 attempts) for one
    // logical send, so this surfaces as a real
    // `SendError { is_recoverable: true }` to the `RoomSendQueue` task
    // instead of being absorbed by HTTP-layer retry.
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

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 45);
    let first = owner
        .send_text(
            room_id.to_string(),
            "first (recoverable failure)".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("a recoverable failure is still queued, not a failed send");
    assert_eq!(
        first.status, "queued",
        "a recoverable error leaves the request queued; reporting it failed invites a duplicate send"
    );
    assert!(first.event_id.is_empty());
    let projected = owner.outbound_text_for_room(room_id.as_str()).await;
    assert_eq!(projected[0].state, LocalEchoState::Sending);
    assert_eq!(projected[0].local_txn_id, first.local_txn_id);
    let first_txn = projected[0].local_txn_id.clone();
    assert!(
        !synara_core::app::send::queued_send_is_wedged(&room, &first_txn)
            .await
            .unwrap_or(true),
        "recoverable failure must not mark the SDK request wedged"
    );

    // The server recovers (matches "HTTP 520 must remain retried"). A
    // brand-new send in the same room must not hang: the SDK disables the
    // room's local send-queue after any error (recoverable or not), and only
    // a wedged (unrecoverable) failure should keep later sends blocked.
    server
        .mock_room_send()
        .ok(event_id!("$after-recoverable"))
        .mount()
        .await;
    let second = tokio::time::timeout(
        Duration::from_secs(10),
        owner.send_text(
            room_id.to_string(),
            "second (fresh message)".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
    )
    .await
    .expect("a fresh send after a recoverable failure must not hang")
    .expect("a fresh send after a recoverable failure must succeed");
    assert_eq!(second.event_id, "$after-recoverable");
    assert_eq!(second.status, "sent");
}

#[tokio::test]
async fn unwedge_retries_wedged_send() {
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

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 43);
    let _ = owner
        .send_text(
            room_id.to_string(),
            "retry me".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("wedge first attempt");
    let txn = owner.outbound_text_for_room(room_id.as_str()).await[0]
        .local_txn_id
        .clone();
    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    server
        .mock_room_send()
        .ok(event_id!("$unwedged"))
        .mock_once()
        .mount()
        .await;
    owner
        .unwedge_send(room_id.as_str(), &txn)
        .await
        .expect("unwedge");
    let sent = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            match watch.recv().await {
                Ok(matrix_sdk::send_queue::RoomSendQueueUpdate::SentEvent {
                    event_id,
                    transaction_id,
                }) if transaction_id.as_str() == txn => {
                    return event_id;
                }
                Ok(_) => continue,
                Err(_) => panic!("send queue closed before unwedge sent"),
            }
        }
    })
    .await
    .expect("unwedge must send");
    assert_eq!(sent.as_str(), "$unwedged");
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

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 44);
    let result = owner
        .send_text(
            room_id.to_string(),
            "cloudflare blip".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await;
    match result {
        Ok(ack) => {
            // `sent` after an HTTP-layer retry, or `queued` when the 520s
            // surfaced as a recoverable send-queue error; never wedged.
            assert!(ack.status == "sent" || ack.status == "queued");
            let projected = owner.outbound_text_for_room(room_id.as_str()).await;
            assert_ne!(projected[0].state, LocalEchoState::Wedged);
            assert!(
                !synara_core::app::send::queued_send_is_wedged(&room, &projected[0].local_txn_id)
                    .await
                    .unwrap_or(false),
                "520 must not mark the SDK request wedged"
            );
        }
        Err(diagnostic) => {
            assert_ne!(diagnostic, "d0.4-send-queue-wedged");
            let projected = owner.outbound_text_for_room(room_id.as_str()).await;
            assert_ne!(projected[0].state, LocalEchoState::Wedged);
            assert!(
                !synara_core::app::send::queued_send_is_wedged(&room, &projected[0].local_txn_id)
                    .await
                    .unwrap_or(true),
                "520 must not mark the SDK request wedged"
            );
        }
    }
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

#[tokio::test]
async fn discarding_a_parked_recoverable_failure_never_sends_it() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-discard-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "errcode": "M_UNKNOWN",
            "error": "transient failure",
        })))
        .mount()
        .await;

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 46);
    let first = owner
        .send_text(
            room_id.to_string(),
            "discard me".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("a persistent 5xx leaves the send queued");
    assert_eq!(first.status, "queued");
    let txn = owner.outbound_text_for_room(room_id.as_str()).await[0]
        .local_txn_id
        .clone();

    // `wait_for_queued_send` re-enabled the room queue, so the SDK retries in
    // the background. Wait until that retry fails too: the request is then
    // parked in the store with the room queue disabled, which is the state a
    // restored unsent echo is in when the user presses Discard.
    tokio::time::timeout(Duration::from_secs(20), async {
        let mut failures = 0;
        loop {
            match watch.recv().await {
                Ok(matrix_sdk::send_queue::RoomSendQueueUpdate::SendError {
                    transaction_id,
                    ..
                }) if transaction_id.as_str() == txn => {
                    failures += 1;
                    if failures >= 2 {
                        return;
                    }
                }
                Ok(_) => continue,
                Err(_) => panic!("send queue closed before the retry failed"),
            }
        }
    })
    .await
    .expect("the background retry must fail and park the request");
    assert!(
        !room.send_queue().is_enabled(),
        "the SDK disables the room queue after a send error"
    );

    let before = room_send_request_count(
        &server
            .server()
            .received_requests()
            .await
            .unwrap_or_default(),
    );
    let aborted = owner
        .abort_send(room_id.as_str(), &txn)
        .await
        .expect("discard parked send");
    assert!(aborted, "a parked request is still local and must abort");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let after = room_send_request_count(
        &server
            .server()
            .received_requests()
            .await
            .unwrap_or_default(),
    );
    assert_eq!(
        after, before,
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
    let projected = owner.outbound_text_for_room(room_id.as_str()).await;
    assert_eq!(projected[0].state, LocalEchoState::Cancelled);
}

#[tokio::test]
async fn discard_after_send_reports_not_aborted_and_keeps_the_sent_state() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-discard-sent:example.org");
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .ok(event_id!("$already-sent"))
        .mock_once()
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 47);
    let sent = owner
        .send_text(
            room_id.to_string(),
            "too late to discard".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("send succeeds");
    // The SDK drops a sent request from its local-echo list, so discard of
    // that transaction cannot abort anything. The product queue must keep
    // the Sent state instead of recording a cancel.
    let result = owner.abort_send(room_id.as_str(), &sent.local_txn_id).await;
    match result {
        Ok(aborted) => assert!(!aborted, "a sent request cannot be aborted"),
        Err(diagnostic) => assert_eq!(diagnostic, "d0.4-send-queue-handle-not-found"),
    }
    let projected = owner.outbound_text_for_room(room_id.as_str()).await;
    assert_eq!(projected[0].state, LocalEchoState::Sent);
}

#[tokio::test]
async fn sync_recovery_resends_a_parked_recoverable_failure() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!send-queue-recovery:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // Two logical attempts (3 HTTP tries each) fail: the first inside the
    // command, the second after `wait_for_queued_send` re-enables the room.
    server
        .mock_room_send()
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "errcode": "M_UNKNOWN",
            "error": "transient failure",
        })))
        .up_to_n_times(6)
        .mount()
        .await;

    let (running_tx, running_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();
    let running = Box::pin(futures_util::stream::unfold(
        running_rx,
        |mut rx| async move { rx.recv().await.map(|value| (value, rx)) },
    ));
    let recovery = synara_core::app::send::spawn_send_queue_recovery(client.clone(), running);
    running_tx.send(true).unwrap();

    let (_echoes, mut watch) = room.send_queue().subscribe().await.expect("subscribe");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 47);
    let first = owner
        .send_text(
            room_id.to_string(),
            "send me after recovery".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("still queued");
    assert_eq!(first.status, "queued");
    let txn = first.local_txn_id.clone();
    tokio::time::timeout(Duration::from_secs(20), async {
        let mut failures = 0;
        loop {
            match watch.recv().await {
                Ok(matrix_sdk::send_queue::RoomSendQueueUpdate::SendError {
                    transaction_id,
                    ..
                }) if transaction_id.as_str() == txn => {
                    failures += 1;
                    if failures >= 2 {
                        return;
                    }
                }
                Ok(_) => continue,
                Err(_) => panic!("send queue closed"),
            }
        }
    })
    .await
    .expect("the background retry fails and parks the request");
    assert!(!room.send_queue().is_enabled());

    // The server recovers and sync reports a recovery edge.
    server
        .mock_room_send()
        .ok(event_id!("$after-recovery"))
        .mount()
        .await;
    running_tx.send(false).unwrap();
    running_tx.send(true).unwrap();
    let sent = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match watch.recv().await {
                Ok(matrix_sdk::send_queue::RoomSendQueueUpdate::SentEvent {
                    transaction_id,
                    event_id,
                }) if transaction_id.as_str() == txn => return event_id,
                Ok(_) => continue,
                Err(_) => panic!("send queue closed"),
            }
        }
    })
    .await
    .expect("sync recovery must re-enable the parked room queue");
    assert_eq!(sent.as_str(), "$after-recovery");
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

    let (running_tx, running_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();
    let running = Box::pin(futures_util::stream::unfold(
        running_rx,
        |mut rx| async move { rx.recv().await.map(|value| (value, rx)) },
    ));
    let recovery = synara_core::app::send::spawn_send_queue_recovery(client.clone(), running);
    running_tx.send(true).unwrap();
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 48);
    let error = owner
        .send_text(
            room_id.to_string(),
            "too big".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("an unrecoverable send is a failure");
    assert_eq!(error, "d0.4-send-queue-wedged");
    let before = room_send_request_count(
        &server
            .server()
            .received_requests()
            .await
            .unwrap_or_default(),
    );
    running_tx.send(false).unwrap();
    running_tx.send(true).unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let after = room_send_request_count(
        &server
            .server()
            .received_requests()
            .await
            .unwrap_or_default(),
    );
    assert_eq!(after, before, "a wedged request waits for Retry or Discard");
    let txn = owner.outbound_text_for_room(room_id.as_str()).await[0]
        .local_txn_id
        .clone();
    assert!(synara_core::app::send::queued_send_is_wedged(&room, &txn)
        .await
        .unwrap());
    recovery.abort();
}
