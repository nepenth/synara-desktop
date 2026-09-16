use super::*;
use matrix_sdk::ruma::{
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
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, TimelineViewPosition, TimelineViewRow,
};

fn row_event_id(row: &TimelineViewRow) -> Option<&str> {
    match row {
        TimelineViewRow::Message(row) => row.event.event_id.as_deref(),
        TimelineViewRow::Poll(row) => row.event.event_id.as_deref(),
        TimelineViewRow::Sticker { event, .. }
        | TimelineViewRow::Membership(crate::app::timeline::TimelineMembershipRow {
            event, ..
        })
        | TimelineViewRow::State(crate::app::timeline::TimelineStateRow { event, .. })
        | TimelineViewRow::Call(crate::app::timeline::TimelineCallRow { event, .. })
        | TimelineViewRow::Redacted(crate::app::timeline::TimelineRedactedRow { event, .. })
        | TimelineViewRow::EncryptedUnavailable(
            crate::app::timeline::TimelineEncryptedUnavailableRow { event, .. },
        ) => event.event_id.as_deref(),
        TimelineViewRow::Other(row) => row.event_id.as_deref(),
        TimelineViewRow::DateSeparator { .. }
        | TimelineViewRow::ReadMarker { .. }
        | TimelineViewRow::UnreadMarker { .. }
        | TimelineViewRow::TimelineStart { .. }
        | TimelineViewRow::Pagination { .. } => None,
    }
}

fn message_thread_root<'a>(row: &'a TimelineViewRow) -> Option<&'a str> {
    match row {
        TimelineViewRow::Message(row) => row.thread_root.as_deref(),
        _ => None,
    }
}

#[tokio::test]
async fn thread_open_uses_a_distinct_threaded_stream_from_live_and_permalink() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!thread-open:example.org");
    let root_id = event_id!("$thread-root");
    let reply_id = event_id!("$thread-reply");
    let live_id = event_id!("$live-plain");
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
                .add_timeline_event(f.event(reply).sender(*BOB).event_id(reply_id))
                .add_timeline_event(f.text_msg("unthreaded live").sender(*BOB).event_id(live_id)),
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
        .events_after(vec![
            f.event({
                let mut reply = RoomMessageEventContent::text_plain("thread reply");
                reply.relates_to = Some(Relation::Thread(Thread::reply(
                    root_id.to_owned(),
                    root_id.to_owned(),
                )));
                reply
            })
            .sender(*BOB)
            .event_id(reply_id)
            .into_event(),
            f.text_msg("unthreaded live")
                .sender(*BOB)
                .event_id(live_id)
                .into_event(),
        ])
        .start("thread-open-prev")
        .end("thread-open-next"))
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 41);
    let live = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        }),
    )
    .await
    .expect("live open should finish")
    .expect("live open must succeed");
    let live_ids: Vec<_> = live.snapshot.rows.iter().filter_map(row_event_id).collect();
    assert!(
        live_ids.contains(&root_id.as_str()),
        "T1 live still shows the thread root: {live_ids:?}"
    );
    assert!(
        live_ids.contains(&reply_id.as_str()),
        "T1 must not hide threaded replies on live: {live_ids:?}"
    );
    assert!(live_ids.contains(&live_id.as_str()));
    assert_eq!(
        owner.debug_live_is_threaded(room_id.as_str()).await,
        Some(false)
    );
    assert_eq!(
        owner.debug_stream_is_threaded(&live.stream_id).await,
        Some(false)
    );
    assert!(
        live.stream_id.contains(&format!("live:{room_id}")),
        "{}",
        live.stream_id
    );

    let focused = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: root_id.to_string(),
            },
        }),
    )
    .await
    .expect("focused open should finish")
    .expect("focused permalink of the root must succeed");
    assert!(
        matches!(
            focused.position,
            TimelineViewPosition::Focused { ref target_event_id } if target_event_id == root_id.as_str()
        ),
        "{:?}",
        focused.position
    );
    assert_eq!(
        owner.debug_stream_is_threaded(&focused.stream_id).await,
        Some(false)
    );

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
        thread.position,
        TimelineViewPosition::Thread {
            root_event_id: root_id.to_string(),
        }
    );
    assert_eq!(
        thread.snapshot.position,
        TimelineViewPosition::Thread {
            root_event_id: root_id.to_string(),
        }
    );
    assert!(
        thread
            .stream_id
            .contains(&format!("thread:{room_id}:{root_id}")),
        "{}",
        thread.stream_id
    );
    assert_ne!(thread.stream_id, focused.stream_id);
    assert_eq!(
        owner.debug_stream_is_threaded(&thread.stream_id).await,
        Some(true)
    );
    let thread_ids: Vec<_> = thread
        .snapshot
        .rows
        .iter()
        .filter_map(row_event_id)
        .collect();
    assert!(
        thread_ids.contains(&reply_id.as_str()),
        "thread timeline must include in-thread replies: {thread_ids:?}"
    );
    assert!(
        !thread_ids.contains(&live_id.as_str()),
        "thread timeline must not include unthreaded live events: {thread_ids:?}"
    );
    assert!(
        thread
            .snapshot
            .rows
            .iter()
            .any(|row| message_thread_root(row) == Some(root_id.as_str())),
        "in-thread rows must project the root id"
    );

    let rejected = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Thread {
                root_event_id: "not-an-event".into(),
            },
        })
        .await
        .expect_err("invalid thread roots fail closed");
    assert_eq!(rejected, "v-timeline-thread-root-invalid");
    assert!(!rejected.contains("not-an-event"));
}
