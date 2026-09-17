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
use ruma::event_id;

use crate::app::timeline::{
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, NativeTimelineReadAction,
    NativeTimelineReadIntent, NativeTimelineReadStateRequest,
};

#[tokio::test]
async fn thread_mark_read_sends_a_private_receipt_not_unthreaded_read_markers() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!thread-receipt:example.org");
    let root_id = event_id!("$thread-root");
    let reply_id = event_id!("$thread-reply");
    let f = EventFactory::new().room(room_id);
    let mut reply = RoomMessageEventContent::text_plain("thread reply");
    reply.relates_to = Some(Relation::Thread(Thread::reply(
        root_id.to_owned(),
        root_id.to_owned(),
    )));
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("thread root").sender(*BOB).event_id(root_id))
                .add_timeline_event(f.event(reply).sender(*BOB).event_id(reply_id)),
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
            .event({
                let mut reply = RoomMessageEventContent::text_plain("thread reply");
                reply.relates_to = Some(Relation::Thread(Thread::reply(
                    root_id.to_owned(),
                    root_id.to_owned(),
                )));
                reply
            })
            .sender(*BOB)
            .event_id(reply_id)
            .into_event()])
        .start("thread-receipt-prev")
        .end("thread-receipt-next"))
        .mount()
        .await;
    server
        .mock_send_receipt(ReceiptType::ReadPrivate)
        .ok()
        .mock_once()
        .mount()
        .await;
    server.mock_send_read_markers().ok().expect(0).mount().await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 11);
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

    let readback = timeout(
        Duration::from_secs(8),
        owner.set_read_state(NativeTimelineReadStateRequest {
            stream_id: thread.stream_id.clone(),
            action: NativeTimelineReadAction::MarkRead,
            intent: NativeTimelineReadIntent::ExplicitUser,
            observed_live_tail_event_id: None,
        }),
    )
    .await
    .expect("thread mark-read should finish")
    .expect("thread mark-read must use the thread stream, not reject as a live-only write");
    assert_eq!(readback.action, NativeTimelineReadAction::MarkRead);
    assert_eq!(readback.receipt_sent, Some(true));
}
