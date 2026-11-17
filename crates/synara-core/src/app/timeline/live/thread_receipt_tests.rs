use super::*;
use matrix_sdk::ruma::{
    api::client::receipt::create_receipt::v3::ReceiptType,
    events::{
        relation::Thread,
        room::message::{Relation, RoomMessageEventContent},
    },
    room_id, RoomVersionId,
};
use matrix_sdk::test_utils::mocks::{
    MatrixMockServer, RoomContextResponseTemplate, RoomMessagesResponseTemplate,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::{event_id, EventId};
use serde_json::json;
use wiremock::ResponseTemplate;

use crate::app::timeline::{
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, NativeTimelineReadAction,
    NativeTimelineReadIntent, NativeTimelineReadStateRequest,
};

fn thread_reply(root_id: &EventId) -> RoomMessageEventContent {
    let mut reply = RoomMessageEventContent::text_plain("thread reply");
    reply.relates_to = Some(Relation::Thread(Thread::reply(
        root_id.to_owned(),
        root_id.to_owned(),
    )));
    reply
}

/// A synced room with a one-reply thread, and a thread stream open on it.
/// Receipt endpoints are left to each test.
async fn open_thread_stream(
    server: &MatrixMockServer,
    client: &Client,
    room_id: &RoomId,
    session_generation: u64,
) -> (Arc<NativeTimelineOwner>, String) {
    let root_id = event_id!("$thread-root");
    let reply_id = event_id!("$thread-reply");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("thread root").sender(*BOB).event_id(root_id))
                .add_timeline_event(
                    f.event(thread_reply(root_id))
                        .sender(*BOB)
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
    server
        .mock_room_event_context()
        .room(room_id)
        .ok(RoomContextResponseTemplate::new(
            f.text_msg("thread root")
                .sender(*BOB)
                .event_id(root_id)
                .into_event(),
        )
        .events_before(vec![])
        .events_after(vec![f
            .event(thread_reply(root_id))
            .sender(*BOB)
            .event_id(reply_id)
            .into_event()])
        .start("thread-receipt-prev")
        .end("thread-receipt-next"))
        .mount()
        .await;

    let owner = Arc::new(NativeTimelineOwner::new(
        client,
        Arc::new(|_| {}),
        session_generation,
    ));
    let thread = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Thread {
                root_event_id: root_id.to_string(),
            },
        }),
    )
    .await
    .expect("thread open should finish")
    .expect("thread open must succeed");
    assert_eq!(
        owner.debug_stream_is_threaded(&thread.stream_id).await,
        Some(true)
    );
    (owner, thread.stream_id)
}

fn explicit_mark_read(stream_id: &str) -> NativeTimelineReadStateRequest {
    NativeTimelineReadStateRequest {
        stream_id: stream_id.to_owned(),
        action: NativeTimelineReadAction::MarkRead,
        intent: NativeTimelineReadIntent::ExplicitUser,
        observed_live_tail_event_id: None,
        public_receipt: false,
    }
}

async fn receipt_requests(server: &MatrixMockServer) -> usize {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().contains("/receipt/"))
        .count()
}

#[tokio::test]
async fn thread_mark_read_sends_a_private_receipt_not_unthreaded_read_markers() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!thread-receipt:example.org");
    let (owner, stream_id) = open_thread_stream(&server, &client, room_id, 11).await;
    server
        .mock_send_receipt(ReceiptType::ReadPrivate)
        .ok()
        .mock_once()
        .mount()
        .await;
    server.mock_send_read_markers().ok().expect(0).mount().await;

    let readback = timeout(
        Duration::from_secs(8),
        owner.set_read_state(explicit_mark_read(&stream_id)),
    )
    .await
    .expect("thread mark-read should finish")
    .expect("thread mark-read must use the thread stream, not reject as a live-only write");
    assert_eq!(readback.action, NativeTimelineReadAction::MarkRead);
    assert_eq!(readback.receipt_sent, Some(true));
}

/// Closing a thread pane closes its stream, often while the pane's last
/// receipt is still on the wire. The receipt went out, so the write is a
/// success, not `v-timeline-view-not-open`. The registry lock is free while
/// the receipt is in flight, so the close does not wait for it.
#[tokio::test]
async fn a_stream_closed_while_its_receipt_is_in_flight_still_reads_back() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!thread-receipt-close:example.org");
    let (owner, stream_id) = open_thread_stream(&server, &client, room_id, 12).await;
    server
        .mock_send_receipt(ReceiptType::ReadPrivate)
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({}))
                .set_delay(Duration::from_secs(2)),
        )
        .mock_once()
        .mount()
        .await;

    let marking = {
        let owner = owner.clone();
        let request = explicit_mark_read(&stream_id);
        tokio::spawn(async move { owner.set_read_state(request).await })
    };
    timeout(Duration::from_secs(5), async {
        while receipt_requests(&server).await == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the receipt request goes out");

    let closed = timeout(Duration::from_millis(750), async {
        owner.lock().await.close_view(NativeTimelineCloseRequest {
            stream_id: stream_id.clone(),
        })
    })
    .await
    .expect("closing the stream must not wait for the receipt");
    assert!(closed);
    assert!(!marking.is_finished(), "the receipt is still in flight");

    let readback = timeout(Duration::from_secs(10), marking)
        .await
        .expect("the receipt finishes")
        .unwrap()
        .expect("a receipt sent before the stream closed is a success");
    assert_eq!(readback.action, NativeTimelineReadAction::MarkRead);
    assert_eq!(readback.receipt_sent, Some(true));
    assert_eq!(readback.snapshot.room_id, room_id.as_str());
    assert_eq!(
        owner.snapshot(&stream_id).await.unwrap_err(),
        "v-timeline-view-not-open",
        "the stream itself stays closed"
    );
}
