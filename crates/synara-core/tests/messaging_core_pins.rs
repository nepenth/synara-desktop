//! Pin panel, mixed receipts, and automatic last-message back-pagination.
//!
//! Pin rows come from `PinnedEventsCache` (not `TimelineFocus::PinnedEvents`).
//! Unpin of a non-final pin must keep the remaining row and its reactions.
//! A thread receipt in the same sync must not starve the main/unthreaded
//! receipt that drives live unread. Idle rooms with no displayable tail
//! back-paginate through `BackPaginationQueue` without opening a timeline.

#![recursion_limit = "256"]

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use matrix_sdk::latest_events::LatestEventValue;
use matrix_sdk::room::IncludeRelations;
use matrix_sdk::ruma::{
    event_id,
    events::{
        receipt::{ReceiptThread, ReceiptType},
        relation::RelationType,
        TimelineEventType,
    },
    room_id, RoomVersionId,
};
use matrix_sdk::test_utils::mocks::{
    MatrixMockServer, RoomMessagesResponseTemplate, RoomRelationsResponseTemplate,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use synara_core::app::room_list::last_message_preview_from_event_json_str;
use synara_core::app::timeline::{NativeTimelineOwner, PINNED_EVENTS_SCHEMA_VERSION};

#[tokio::test]
async fn pin_panel_keeps_remaining_reactions_after_unpin() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!pins-core:example.org");
    let pin_a = event_id!("$pinned-a");
    let pin_b = event_id!("$pinned-b");
    let react_a = event_id!("$react-a");
    let react_b = event_id!("$react-b");
    let f = EventFactory::new().room(room_id).sender(*BOB);
    let own_user_id = client.user_id().unwrap().to_owned();
    let mut pin_power = BTreeMap::from([(own_user_id.clone(), 100.into())]);

    let event_a = f
        .text_msg("pinned one")
        .sender(*BOB)
        .event_id(pin_a)
        .into_event();
    let event_b = f
        .text_msg("pinned two")
        .sender(*BOB)
        .event_id(pin_b)
        .into_event();

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .add_state_event(f.power_levels(&mut pin_power).state_key(""))
                .add_state_bulk(vec![f
                    .room_pinned_events(vec![pin_a.to_owned(), pin_b.to_owned()])
                    .into()])
                .add_timeline_event(f.text_msg("pinned one").sender(*BOB).event_id(pin_a))
                .add_timeline_event(f.reaction(pin_a, "👍").sender(*BOB).event_id(react_a))
                .add_timeline_event(f.text_msg("pinned two").sender(*BOB).event_id(pin_b))
                .add_timeline_event(f.reaction(pin_b, "🔥").sender(*BOB).event_id(react_b)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    server
        .mock_room_event()
        .match_event_id()
        .ok(event_a.clone())
        .mount()
        .await;
    server
        .mock_room_event()
        .match_event_id()
        .ok(event_b.clone())
        .mount()
        .await;
    server
        .mock_room_relations()
        .match_target_event(pin_a.to_owned())
        .match_subrequest(IncludeRelations::RelationsOfTypeAndEventType(
            RelationType::Annotation,
            TimelineEventType::Reaction,
        ))
        .ok(RoomRelationsResponseTemplate::default().events(vec![f
            .reaction(pin_a, "👍")
            .sender(*BOB)
            .event_id(react_a)
            .into_raw_timeline()]))
        .mount()
        .await;
    server
        .mock_room_relations()
        .match_target_event(pin_b.to_owned())
        .match_subrequest(IncludeRelations::RelationsOfTypeAndEventType(
            RelationType::Annotation,
            TimelineEventType::Reaction,
        ))
        .ok(RoomRelationsResponseTemplate::default().events(vec![f
            .reaction(pin_b, "🔥")
            .sender(*BOB)
            .event_id(react_b)
            .into_raw_timeline()]))
        .mount()
        .await;
    server
        .mock_set_room_pinned_events()
        .ok(event_id!("$unpin-state").to_owned())
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 51);
    let first = wait_for_pins(&owner, room_id.as_str(), |snapshot| {
        snapshot.items.len() == 2
    })
    .await;
    assert_eq!(first.schema_version, PINNED_EVENTS_SCHEMA_VERSION);
    assert_eq!(first.event_ids, vec![pin_a.to_string(), pin_b.to_string()]);
    assert_eq!(first.items[0].body.as_deref(), Some("pinned one"));
    assert_eq!(first.items[1].body.as_deref(), Some("pinned two"));
    assert_eq!(first.items[0].reactions[0].key, "👍");
    assert_eq!(
        first.items[0].reactions[0].senders[0]
            .reaction_event_id
            .as_deref(),
        Some(react_a.as_str())
    );
    assert_eq!(first.items[1].reactions[0].key, "🔥");
    assert_eq!(
        first.items[1].reactions[0].senders[0]
            .reaction_event_id
            .as_deref(),
        Some(react_b.as_str())
    );

    let unpinned = owner
        .unpin_event(room_id.as_str(), pin_a.as_str())
        .await
        .expect("unpin remaining pin");
    assert_eq!(unpinned.status, "unpinned");

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_bulk(vec![f.room_pinned_events(vec![pin_b.to_owned()]).into()]),
        )
        .await;

    let remaining = wait_for_pins(&owner, room_id.as_str(), |snapshot| {
        snapshot.items.len() == 1
    })
    .await;
    assert_eq!(remaining.event_ids, vec![pin_b.to_string()]);
    assert_eq!(remaining.items[0].event_id, pin_b.as_str());
    assert_eq!(remaining.items[0].reactions[0].key, "🔥");
    assert_eq!(
        remaining.items[0].reactions[0].senders[0]
            .reaction_event_id
            .as_deref(),
        Some(react_b.as_str())
    );
}

#[tokio::test]
async fn mixed_thread_and_unthreaded_receipts_follow_main_unread() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!receipts-mixed:example.org");
    let own_user_id = client.user_id().unwrap().to_owned();
    let thread_root = event_id!("$thread-root");
    let thread_reply = event_id!("$thread-reply");
    let main_later = event_id!("$main-later");
    let f = EventFactory::new().room(room_id);

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .add_timeline_event(f.text_msg("root").sender(*BOB).event_id(thread_root))
                .add_timeline_event(
                    f.text_msg("in thread")
                        .sender(*BOB)
                        .event_id(thread_reply)
                        .in_thread(thread_root, thread_root),
                )
                .add_timeline_event(f.text_msg("later main").sender(*BOB).event_id(main_later)),
        )
        .await;

    let room = client.get_room(room_id).expect("joined room");
    let unread_before = room.num_unread_messages();
    assert!(
        unread_before > 0,
        "bob's main timeline messages start unread"
    );

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_receipt(
                f.read_receipts()
                    .add(
                        thread_reply,
                        &own_user_id,
                        ReceiptType::Read,
                        ReceiptThread::Thread(thread_root.to_owned()),
                    )
                    .add(
                        main_later,
                        &own_user_id,
                        ReceiptType::Read,
                        ReceiptThread::Unthreaded,
                    )
                    .into_event(),
            ),
        )
        .await;

    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if room.num_unread_messages() == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("unthreaded receipt on the live tail must clear main unread even with a thread receipt in the same sync");
    assert_eq!(room.num_unread_notifications(), 0);
}

#[tokio::test]
async fn automatic_back_pagination_fills_empty_last_message_preview() {
    let server = MatrixMockServer::new().await;
    let client = server
        .client_builder()
        .on_builder(|builder| builder.with_enable_automatic_back_pagination(true))
        .build()
        .await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!idle-preview:example.org");
    let own_user_id = client.user_id().unwrap().to_owned();
    let hidden = event_id!("$behind-gap");
    let f = EventFactory::new().room(room_id);

    server
        .mock_room_messages()
        .match_from("idle-prev")
        .ok(RoomMessagesResponseTemplate::default().events(vec![f
            .text_msg("hello from history")
            .sender(*BOB)
            .event_id(hidden)]))
        .mount()
        .await;

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .set_timeline_limited()
                .set_timeline_prev_batch("idle-prev"),
        )
        .await;

    let room = client.get_room(room_id).expect("idle room");
    let latest_events = client.latest_events().await;
    latest_events
        .listen_to_room(room_id)
        .await
        .expect("listen for automatic latest-event backfill");

    let preview = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let LatestEventValue::Remote(event) = room.latest_event() {
                return last_message_preview_from_event_json_str(event.raw().json().get());
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    })
    .await
    .expect(
        "automatic back-pagination must surface a remote last-message without opening the timeline",
    );
    assert_eq!(preview.as_deref(), Some("hello from history"));
}

#[test]
fn pin_and_pagination_source_uses_sdk_owners() {
    let pins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/timeline/pins.rs"
    ));
    assert!(pins.contains("pinned_events("));
    assert!(pins.contains("PinnedEventsCache"));
    let invented_focus = concat!("TimelineFocus::", "PinnedEvents");
    assert!(!pins.contains(invented_focus));

    let open = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/client_builder/open.rs"
    ));
    assert!(open.contains("with_enable_automatic_back_pagination(true)"));

    let live = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/timeline/live.rs"
    ));
    assert!(live.contains("client.event_cache().subscribe()"));

    let history = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/timeline/live/approval_history.rs"
    ));
    assert!(history.contains("back_pagination_queue()"));
}

async fn wait_for_pins(
    owner: &NativeTimelineOwner,
    room_id: &str,
    accept: impl Fn(&synara_core::app::timeline::PinnedEventsSnapshot) -> bool,
) -> synara_core::app::timeline::PinnedEventsSnapshot {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let snapshot = owner
                .pinned_events_snapshot(room_id)
                .await
                .expect("pinned snapshot");
            if accept(&snapshot) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    })
    .await
    .expect("pinned events cache must project native rows")
}
