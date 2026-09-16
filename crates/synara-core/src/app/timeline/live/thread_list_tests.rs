use super::*;
use matrix_sdk::ruma::{room_id, RoomVersionId};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::event_id;

#[tokio::test]
async fn thread_list_open_projects_ids_and_counts_without_tokens() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!thread-list:example.org");
    let root_id = event_id!("$thread-root");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11)),
        )
        .await;
    server
        .mock_room_threads()
        .ok(
            vec![f
                .text_msg("thread root")
                .sender(*BOB)
                .event_id(root_id)
                .into_raw()],
            None,
        )
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 7);
    let snapshot = timeout(
        Duration::from_secs(8),
        owner.thread_list(room_id.as_str(), "open"),
    )
    .await
    .expect("thread list open should finish")
    .expect("thread list open must succeed");
    assert_eq!(snapshot.room_id, room_id.as_str());
    assert_eq!(snapshot.schema_version, 1);
    assert!(snapshot.end_reached);
    assert!(!snapshot.truncated);
    assert_eq!(snapshot.threads.len(), 1);
    assert_eq!(snapshot.threads[0].root_event_id, root_id.as_str());
    assert_eq!(snapshot.threads[0].reply_count, 0);
    assert!(
        snapshot.threads[0].unread_count.is_none() || snapshot.threads[0].unread_count == Some(0)
    );
    let json = serde_json::to_value(&snapshot).unwrap();
    let encoded = json.to_string();
    assert!(!encoded.contains("prev_batch"));
    assert!(!encoded.contains("next_batch"));

    let closed = owner
        .thread_list(room_id.as_str(), "close")
        .await
        .expect("close must succeed");
    assert!(closed.threads.is_empty());
    assert!(owner
        .thread_list("not-a-room", "open")
        .await
        .expect_err("invalid room ids fail closed")
        .contains("invalid-room"));
}
