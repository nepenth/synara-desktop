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

fn message_thread_root(row: &TimelineViewRow) -> Option<&str> {
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
        "live still shows the thread root: {live_ids:?}"
    );
    assert!(
        !live_ids.contains(&reply_id.as_str()),
        "live must hide threaded replies once the thread view owner exists: {live_ids:?}"
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

#[tokio::test]
async fn decryption_retry_requests_the_exact_sdk_stream_and_rejects_closed_streams() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!retry:example.org");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(f.text_msg("Already decrypted").sender(*BOB)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 41);
    assert_eq!(
        owner.retry_decryption("unknown").await,
        Err("v-timeline-view-not-open")
    );
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .expect("open SDK timeline");
    assert!(owner
        .retry_decryption(&opened.stream_id)
        .await
        .expect("SDK retry requested"));
    assert_eq!(
        owner.snapshot(&opened.stream_id).await.unwrap().rows,
        opened.snapshot.rows
    );
    owner
        .lock()
        .await
        .close_view(crate::app::timeline::NativeTimelineCloseRequest {
            stream_id: opened.stream_id.clone(),
        });
    assert_eq!(
        owner.retry_decryption(&opened.stream_id).await,
        Err("v-timeline-view-not-open")
    );
}

#[tokio::test]
async fn decryption_retry_acknowledges_request_and_live_rows_only_recover_after_keys_arrive() {
    use matrix_sdk_crypto::olm::Account;
    use ruma::{device_id, serde::Raw};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let room_id = room_id!("!retry-encrypted:example.org");
    let sender = Account::with_device_id(*BOB, device_id!("FIXTURE"));
    let (outbound, inbound) = sender
        .create_group_session_pair_with_defaults(room_id)
        .await;
    let payload = Raw::new(&RoomMessageEventContent::text_plain(
        "Recovered fixture message",
    ))
    .unwrap()
    .cast();
    let encrypted = outbound.encrypt("m.room.message", &payload).await;
    let encrypted: ruma::events::room::encrypted::RoomEncryptedEventContent =
        serde_json::from_str(encrypted.content.json().get()).unwrap();
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().logged_in_with_oauth().build().await;
    client.event_cache().subscribe().unwrap();
    let f = EventFactory::new().room(room_id);
    let event_id = event_id!("$retry-encrypted-fixture");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(f.event(encrypted).sender(*BOB).event_id(event_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let invalidations = Arc::new(AtomicUsize::new(0));
    let emitted = invalidations.clone();
    let owner = NativeTimelineOwner::new(
        &client,
        Arc::new(move |_| {
            emitted.fetch_add(1, Ordering::SeqCst);
        }),
        41,
    );
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .unwrap();
    assert!(opened
        .snapshot
        .rows
        .iter()
        .any(|row| matches!(row, TimelineViewRow::EncryptedUnavailable(_))));
    assert!(owner.retry_decryption(&opened.stream_id).await.unwrap());
    let before_keys = owner.snapshot(&opened.stream_id).await.unwrap();
    assert!(
        before_keys
            .rows
            .iter()
            .any(|row| matches!(row, TimelineViewRow::EncryptedUnavailable(_))),
        "acknowledgement must not manufacture decrypted content"
    );
    let emitted_before = invalidations.load(Ordering::SeqCst);
    client
        .olm_machine_for_testing()
        .await
        .as_ref()
        .unwrap()
        .store()
        .import_exported_room_keys(vec![inbound.export().await], |_, _| {})
        .await
        .unwrap();
    assert!(owner.retry_decryption(&opened.stream_id).await.unwrap());
    let recovered = timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = owner.snapshot(&opened.stream_id).await.unwrap();
            if snapshot.rows.iter().any(|row| {
                matches!(row, TimelineViewRow::Message(message)
                if message.event.event_id.as_deref() == Some(event_id.as_str()))
            }) {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("SDK must project newly decryptable history through the existing stream");
    let message = recovered
        .rows
        .iter()
        .find_map(|row| match row {
            TimelineViewRow::Message(message)
                if message.event.event_id.as_deref() == Some(event_id.as_str()) =>
            {
                Some(message)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(message.body, "Recovered fixture message");
    assert!(invalidations.load(Ordering::SeqCst) > emitted_before);
    assert!(!recovered
        .rows
        .iter()
        .any(|row| matches!(row, TimelineViewRow::EncryptedUnavailable(_))));
}
