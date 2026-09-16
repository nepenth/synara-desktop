use super::*;
use matrix_sdk::ruma::{room_id, RoomVersionId};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};

#[tokio::test]
async fn approval_history_first_view_initializes_a_fresh_clients_event_cache() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let id = room_id!("!fresh-first-view:example.org");
    let f = EventFactory::new().room(id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("cached conversation").sender(*BOB)),
        )
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    // Production session subscribe is `NativeTimelineOwner::new`. This still
    // proves first view does not require a prior test-only subscribe.
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 21);
    let opened = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        }),
    )
    .await
    .expect("first view should finish without external cache initialization")
    .expect("the history gate must initialize its SDK cache prerequisite");
    assert_eq!(opened.snapshot.room_id, id.as_str());
    let mut registry = owner.registry.lock().await;
    let history = registry.approval_history.room(id.as_str()).unwrap();
    assert!(history.protected());
    assert_eq!(registry.view_streams.len(), 1);
    assert!(registry.close_view(NativeTimelineCloseRequest {
        stream_id: opened.stream_id,
    }));
    assert!(!history.protected());
}

#[tokio::test]
async fn approval_history_cancelled_open_releases_unpublished_view_ownership() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!cancelled-first-view:example.org");
    let f = EventFactory::new().room(id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .set_timeline_prev_batch("earlier")
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("sparse cached tail").sender(*BOB)),
        )
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().with_delay(Duration::from_secs(2)))
        .mount()
        .await;
    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 22));
    let history = owner
        .registry
        .lock()
        .await
        .approval_history
        .room(id.as_str())
        .unwrap();
    let opening = {
        let owner = owner.clone();
        tokio::spawn(async move {
            owner
                .open_at(NativeTimelineOpenRequest {
                    room_id: id.to_string(),
                    position: NativeTimelineOpenPosition::LiveBottom,
                })
                .await
        })
    };
    timeout(Duration::from_secs(3), async {
        loop {
            let requests = server.server().received_requests().await.unwrap();
            if requests
                .iter()
                .any(|request| request.url.path().ends_with("/messages"))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the protected open must reach awaited sparse-history initialization");
    assert!(history.protected());
    assert!(!opening.is_finished());
    opening.abort();
    assert!(opening.await.unwrap_err().is_cancelled());
    let registry = owner.registry.lock().await;
    assert!(
        !history.protected(),
        "cancelled open must drop its local lease"
    );
    assert!(registry.view_streams.is_empty());
    assert!(registry.view_revisions.is_empty());
    assert!(registry.view_update_tasks.is_empty());
}

#[test]
fn approval_history_stream_publication_has_no_cancellation_point_before_readback() {
    // The media/snapshot awaits are internal SDK/store operations, with no
    // external test latch. Enforce the critical final publication invariant
    // alongside the real cancellation test above.
    let source = include_str!("../live.rs");
    let open = source
        .split("    pub async fn open_at(")
        .nth(2)
        .expect("registry open_at")
        .split("    pub async fn snapshot(")
        .next()
        .unwrap();
    let (_, published) = open
        .split_once("self.view_streams.insert(")
        .expect("the ready view is published");
    assert!(!published.contains(".await"));
    assert!(published.contains("Ok(NativeTimelineOpenReadback"));
}
