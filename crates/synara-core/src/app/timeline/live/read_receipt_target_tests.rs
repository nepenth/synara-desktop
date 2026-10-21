//! Which event a read write targets and which receipt key it uses.
//!
//! S10: rooms whose newest activity is a thread reply must clear when the user
//! reads the live bottom, where the reply shows through its root's thread
//! summary. S11: the receipt key follows the user's receipt privacy mode
//! (public `m.read` unless read receipts are hidden).

use super::*;
use matrix_sdk::ruma::{
    api::client::receipt::create_receipt::v3::ReceiptType,
    events::{
        receipt::{ReceiptThread, ReceiptType as EventReceiptType},
        relation::Thread,
        room::message::{Relation, RoomMessageEventContent},
    },
    room_id, RoomVersionId,
};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::{event_id, EventId};

use crate::app::timeline::{
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, NativeTimelineReadAction,
    NativeTimelineReadIntent, NativeTimelineReadStateRequest,
};

const READ_MARKERS_PATH_SUFFIX: &str = "/read_markers";

fn thread_reply(root: &EventId, body: &str) -> RoomMessageEventContent {
    let mut reply = RoomMessageEventContent::text_plain(body);
    reply.relates_to = Some(Relation::Thread(Thread::without_fallback(root.to_owned())));
    reply
}

/// A synced room whose newest activity is a thread reply under a root in the
/// live window, plus a live-bottom stream on it.
async fn room_with_thread_activity(
    server: &MatrixMockServer,
    client: &Client,
    room_id: &RoomId,
    extra: impl FnOnce(&EventFactory, JoinedRoomBuilder) -> JoinedRoomBuilder,
) -> (NativeTimelineOwner, String) {
    let f = EventFactory::new().room(room_id);
    let root_id = event_id!("$s10-root");
    let reply_id = event_id!("$s10-reply");
    let builder = JoinedRoomBuilder::new(room_id)
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
        .add_timeline_event(f.text_msg("root").sender(*BOB).event_id(root_id))
        .add_timeline_event(
            f.event(thread_reply(root_id, "reply"))
                .sender(*BOB)
                .event_id(reply_id),
        );
    server.sync_room(client, extra(&f, builder)).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let owner = NativeTimelineOwner::new(client, Arc::new(|_| {}), 31);
    let opened = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        }),
    )
    .await
    .expect("open should finish")
    .expect("live open");
    (owner, opened.stream_id)
}

async fn automatic_read(
    owner: &NativeTimelineOwner,
    stream_id: &str,
    observed_tail: &str,
    public_receipt: bool,
) -> NativeTimelineReadStateReadback {
    timeout(
        Duration::from_secs(8),
        owner.set_read_state(NativeTimelineReadStateRequest {
            stream_id: stream_id.to_owned(),
            action: NativeTimelineReadAction::MarkRead,
            intent: NativeTimelineReadIntent::AutomaticVisibility,
            observed_live_tail_event_id: Some(observed_tail.to_owned()),
            public_receipt,
        }),
    )
    .await
    .expect("read should finish")
    .expect("automatic read")
}

async fn read_marker_bodies(server: &MatrixMockServer) -> Vec<serde_json::Value> {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|request| request.url.path().ends_with(READ_MARKERS_PATH_SUFFIX))
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

#[tokio::test]
async fn automatic_read_at_the_bottom_covers_thread_replies_its_root_summarizes() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s10-covered:example.org");
    let (owner, stream_id) =
        room_with_thread_activity(&server, &client, room_id, |_, builder| builder).await;
    let room = client.get_room(room_id).unwrap();
    assert!(
        room.num_unread_messages() > 0,
        "the thread reply makes the room unread (threading support stays off)"
    );
    server.mock_send_read_markers().ok().mount().await;

    let readback = automatic_read(&owner, &stream_id, "$s10-root", false).await;

    assert_eq!(readback.receipt_sent, Some(true));
    assert_eq!(
        readback.acknowledged_event_id.as_deref(),
        Some("$s10-root"),
        "shells still see the tail they observed"
    );
    let bodies = read_marker_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0]["m.read.private"], "$s10-reply");
    assert_eq!(bodies[0]["m.fully_read"], "$s10-reply");
    assert!(bodies[0].get("m.read").is_none());

    // The homeserver echoes the receipt; the room's unread counts (desktop
    // row and iOS row alike read them) clear.
    let own = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_receipt(
                f.read_receipts()
                    .add(
                        event_id!("$s10-reply"),
                        &own,
                        EventReceiptType::ReadPrivate,
                        ReceiptThread::Unthreaded,
                    )
                    .into_event(),
            ),
        )
        .await;
    timeout(Duration::from_secs(5), async {
        while room.num_unread_messages() > 0 || room.num_unread_notifications() > 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("room unread counts clear once the receipt covers the thread reply");

    // A repeat observation of the same tail does not post again.
    let repeat = automatic_read(&owner, &stream_id, "$s10-root", false).await;
    assert_eq!(repeat.receipt_sent, Some(true));
    assert_eq!(read_marker_bodies(&server).await.len(), 1);
}

#[tokio::test]
async fn a_receipt_on_the_main_tail_alone_leaves_the_room_unread() {
    // The pre-fix behaviour, pinned so the S10 test above proves the change.
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s10-main-only:example.org");
    let (_owner, _stream_id) =
        room_with_thread_activity(&server, &client, room_id, |_, builder| builder).await;
    let own = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_receipt(
                f.read_receipts()
                    .add(
                        event_id!("$s10-root"),
                        &own,
                        EventReceiptType::ReadPrivate,
                        ReceiptThread::Unthreaded,
                    )
                    .into_event(),
            ),
        )
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let room = client.get_room(room_id).unwrap();
    assert_eq!(room.num_unread_messages(), 1);
}

#[tokio::test]
async fn automatic_read_keeps_the_tail_when_a_newer_reply_has_no_loaded_root() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s10-unloaded-root:example.org");
    let (owner, stream_id) = room_with_thread_activity(&server, &client, room_id, |f, builder| {
        // A reply to a root this timeline never loaded: its summary is not
        // on screen, so the reply stays unread.
        builder.add_timeline_event(
            f.event(thread_reply(event_id!("$s10-old-root"), "old thread"))
                .sender(*BOB)
                .event_id(event_id!("$s10-old-reply")),
        )
    })
    .await;
    server.mock_send_read_markers().ok().mount().await;

    automatic_read(&owner, &stream_id, "$s10-root", false).await;

    let bodies = read_marker_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0]["m.read.private"], "$s10-root");
}

#[tokio::test]
async fn shared_read_receipts_send_public_m_read() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s11-public:example.org");
    let (owner, stream_id) =
        room_with_thread_activity(&server, &client, room_id, |_, builder| builder).await;
    server.mock_send_read_markers().ok().mount().await;

    automatic_read(&owner, &stream_id, "$s10-root", true).await;
    timeout(
        Duration::from_secs(8),
        owner.set_room_read_state(room_id.as_str(), NativeTimelineReadAction::MarkRead, true),
    )
    .await
    .expect("room mark-read should finish")
    .expect("room mark-read");

    let bodies = read_marker_bodies(&server).await;
    assert_eq!(bodies.len(), 2);
    for body in &bodies {
        assert_eq!(body["m.read"], "$s10-reply");
        assert!(
            body.get("m.read.private").is_none(),
            "exactly one receipt key"
        );
    }
}

#[tokio::test]
async fn thread_streams_send_the_receipt_type_of_the_privacy_mode() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s11-thread:example.org");
    let (owner, _live) =
        room_with_thread_activity(&server, &client, room_id, |_, builder| builder).await;
    server
        .mock_room_event_context()
        .room(room_id)
        .ok(
            matrix_sdk::test_utils::mocks::RoomContextResponseTemplate::new(
                EventFactory::new()
                    .room(room_id)
                    .text_msg("root")
                    .sender(*BOB)
                    .event_id(event_id!("$s10-root"))
                    .into_event(),
            )
            .events_before(vec![])
            .events_after(vec![EventFactory::new()
                .room(room_id)
                .event(thread_reply(event_id!("$s10-root"), "reply"))
                .sender(*BOB)
                .event_id(event_id!("$s10-reply"))
                .into_event()])
            .start("s11-prev")
            .end("s11-next"),
        )
        .mount()
        .await;
    server
        .mock_send_receipt(ReceiptType::Read)
        .ok()
        .mock_once()
        .mount()
        .await;
    server
        .mock_send_receipt(ReceiptType::ReadPrivate)
        .ok()
        .expect(0)
        .mount()
        .await;
    let thread = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Thread {
                root_event_id: "$s10-root".to_owned(),
            },
        }),
    )
    .await
    .expect("thread open should finish")
    .expect("thread open");

    let readback = timeout(
        Duration::from_secs(8),
        owner.set_read_state(NativeTimelineReadStateRequest {
            stream_id: thread.stream_id,
            action: NativeTimelineReadAction::MarkRead,
            intent: NativeTimelineReadIntent::ExplicitUser,
            observed_live_tail_event_id: None,
            public_receipt: true,
        }),
    )
    .await
    .expect("thread read should finish")
    .expect("thread read");
    assert_eq!(readback.receipt_sent, Some(true));
}
