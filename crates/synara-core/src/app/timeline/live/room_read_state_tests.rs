//! Explicit room mark-read must move `m.fully_read` for the current remote
//! tail, or fail visibly. Clearing only the local unread flag is success only
//! when the room has no receipt-capable remote event at all.

use super::*;
use matrix_sdk::ruma::{room_id, RoomVersionId};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::event_id;
use serde_json::json;
use wiremock::ResponseTemplate;

use crate::app::timeline::NativeTimelineReadAction;

fn forbidden() -> ResponseTemplate {
    ResponseTemplate::new(403).set_body_json(json!({
        "errcode": "M_FORBIDDEN",
        "error": "denied",
    }))
}

#[tokio::test]
async fn explicit_mark_read_on_a_cold_timeline_pages_to_the_remote_tail() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cold-mark-read:example.org");
    let latest_id = event_id!("$cold-latest");
    let f = EventFactory::new().room(room_id);
    // The room is known from sync, but this process has no timeline events
    // for it: the event cache is cold.
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // The open-time page fails. Opening swallows that so cached rows can
    // still render, which used to leave explicit mark-read with no target.
    server
        .mock_room_messages()
        .respond_with(forbidden())
        .mock_once()
        .mount()
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().events(vec![f
            .text_msg("latest")
            .sender(*BOB)
            .event_id(latest_id)
            .into_raw_timeline()]))
        .mount()
        .await;
    server.mock_send_read_markers().ok().expect(1).mount().await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 21);
    let readback = timeout(
        Duration::from_secs(15),
        owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead),
    )
    .await
    .expect("cold mark-read should finish")
    .expect("cold mark-read must find the remote tail");
    assert!(
        readback.receipt_sent,
        "a room with remote events must not fall back to a local unread-flag clear"
    );
    assert_eq!(
        readback.acknowledged_event_id.as_deref(),
        Some(latest_id.as_str())
    );
    assert!(!readback.unread_flag_cleared);
}

#[tokio::test]
async fn explicit_mark_read_fails_when_history_cannot_load() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cold-mark-read-fails:example.org");
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .respond_with(forbidden())
        .mount()
        .await;
    server.mock_send_read_markers().ok().expect(0).mount().await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 22);
    let result = timeout(
        Duration::from_secs(15),
        owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead),
    )
    .await
    .expect("failed mark-read should finish");
    assert_eq!(
        result,
        Err("v-rooms-room-read-state-mark-read-failed"),
        "a timeline that failed to load is an error, not a silent flag clear"
    );
}

#[tokio::test]
async fn explicit_mark_read_with_no_remote_events_clears_only_the_unread_flag() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!empty-mark-read:example.org");
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // No events and no `end` token: the start of the room.
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    server.mock_send_read_markers().ok().expect(0).mount().await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 23);
    let readback = timeout(
        Duration::from_secs(15),
        owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead),
    )
    .await
    .expect("empty mark-read should finish")
    .expect("an empty room clears its unread flag");
    assert!(!readback.receipt_sent);
    assert!(readback.unread_flag_cleared);
    assert_eq!(readback.acknowledged_event_id, None);
}

#[tokio::test]
async fn explicit_mark_read_surfaces_a_read_marker_failure() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!marker-fails:example.org");
    let latest_id = event_id!("$marker-fails-latest");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("latest").sender(*BOB).event_id(latest_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    server
        .mock_send_read_markers()
        .respond_with(forbidden())
        .expect(1..)
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 24);
    let result = timeout(
        Duration::from_secs(15),
        owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead),
    )
    .await
    .expect("failed marker write should finish");
    assert_eq!(result, Err("v-rooms-room-read-state-mark-read-failed"));
}

#[tokio::test]
async fn explicit_mark_read_targets_the_newest_thread_reply_and_writes_every_time() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!thread-tail:example.org");
    let root_id = event_id!("$thread-root");
    let reply_id = event_id!("$thread-reply");
    let f = EventFactory::new().room(room_id);
    // The newest activity is an in-thread reply. The live timeline hides it,
    // but room unread counts include it.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("root").sender(*BOB).event_id(root_id))
                .add_timeline_event(
                    f.text_msg("reply")
                        .sender(*BOB)
                        .in_thread(root_id, root_id)
                        .event_id(reply_id),
                ),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    // Both clicks must reach the homeserver. Receipt deduplication against the
    // main-timeline view must not turn the second click into a silent no-op.
    server.mock_send_read_markers().ok().expect(2).mount().await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 25);
    for attempt in 0..2 {
        let readback = timeout(
            Duration::from_secs(15),
            owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead),
        )
        .await
        .expect("thread-tail mark-read should finish")
        .expect("thread-tail mark-read should succeed");
        assert!(
            readback.receipt_sent,
            "attempt {attempt} must send receipts"
        );
        assert_eq!(
            readback.acknowledged_event_id.as_deref(),
            Some(reply_id.as_str()),
            "attempt {attempt} must acknowledge the newest thread reply"
        );
    }
}

#[tokio::test]
async fn explicit_mark_read_does_not_hold_the_registry_lock_during_the_receipt_write() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!slow-marker:example.org");
    let latest_id = event_id!("$slow-marker-latest");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("latest").sender(*BOB).event_id(latest_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    server
        .mock_send_read_markers()
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({}))
                .set_delay(Duration::from_secs(2)),
        )
        .expect(1)
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 26);
    let mark = owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead);
    let probe = async {
        // Let mark-read open the timeline and reach the delayed receipt write.
        tokio::time::sleep(Duration::from_millis(500)).await;
        timeout(Duration::from_millis(500), owner.lock())
            .await
            .map(|_guard| ())
    };
    let (readback, probe) = tokio::join!(mark, probe);
    assert!(
        probe.is_ok(),
        "another timeline command must get the registry while receipts are in flight"
    );
    assert!(readback.expect("slow mark-read succeeds").receipt_sent);
}
