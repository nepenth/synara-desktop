//! `Focused` opens of routed events: notifications, permalinks, search hits
//! and reply jumps. A thread reply must never come back as a thread-only
//! stream labelled as a room position, and a target in the live window must
//! stay on the live provider.

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
use wiremock::ResponseTemplate;

use crate::app::timeline::{
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, TimelineViewPosition, TimelineViewRow,
};

fn row_event_ids(rows: &[TimelineViewRow]) -> Vec<&str> {
    rows.iter()
        .filter_map(|row| match row {
            TimelineViewRow::Message(row) => row.event.event_id.as_deref(),
            _ => None,
        })
        .collect()
}

fn thread_reply(root_id: &matrix_sdk::ruma::EventId, body: &str) -> RoomMessageEventContent {
    let mut reply = RoomMessageEventContent::text_plain(body);
    reply.relates_to = Some(Relation::Thread(Thread::reply(
        root_id.to_owned(),
        root_id.to_owned(),
    )));
    reply
}

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn focused_plan_keeps_live_window_targets_on_the_live_provider() {
    let live = ids(&["$a", "$root", "$tail"]);
    assert_eq!(
        plan_focused_open("$a", NativeTimelineEventRoute::Room, &live),
        FocusedOpenPlan::Live {
            anchor_event_id: Some("$a".into()),
            route: NativeTimelineEventRoute::Room,
        }
    );
    // The live window proves a room event even when `/event` was unreachable.
    assert_eq!(
        plan_focused_open("$tail", NativeTimelineEventRoute::Unreachable, &live),
        FocusedOpenPlan::Live {
            anchor_event_id: Some("$tail".into()),
            route: NativeTimelineEventRoute::Room,
        }
    );
    assert_eq!(
        plan_focused_open("$old", NativeTimelineEventRoute::Room, &live),
        FocusedOpenPlan::EventFocused
    );
    assert_eq!(
        plan_focused_open("$old", NativeTimelineEventRoute::Unreachable, &live),
        FocusedOpenPlan::EventFocused
    );
}

#[test]
fn focused_plan_routes_thread_replies_and_missing_targets_to_the_live_room() {
    let live = ids(&["$a", "$root", "$tail"]);
    assert_eq!(
        plan_focused_open(
            "$reply",
            NativeTimelineEventRoute::Thread {
                root_event_id: "$root".into()
            },
            &live
        ),
        FocusedOpenPlan::Live {
            anchor_event_id: Some("$root".into()),
            route: NativeTimelineEventRoute::Thread {
                root_event_id: "$root".into()
            },
        }
    );
    assert_eq!(
        plan_focused_open(
            "$reply",
            NativeTimelineEventRoute::Thread {
                root_event_id: "$old-root".into()
            },
            &live
        ),
        FocusedOpenPlan::Live {
            anchor_event_id: None,
            route: NativeTimelineEventRoute::Thread {
                root_event_id: "$old-root".into()
            },
        }
    );
    assert_eq!(
        plan_focused_open("$gone", NativeTimelineEventRoute::Missing, &live),
        FocusedOpenPlan::Live {
            anchor_event_id: None,
            route: NativeTimelineEventRoute::Missing,
        }
    );
}

#[test]
fn a_thread_root_routes_to_the_room_and_a_reply_to_its_thread() {
    let root = event_id!("$route-root");
    let reply = event_id!("$route-reply");
    assert_eq!(
        event_route_for_thread_root(reply, Some(root.to_owned())),
        NativeTimelineEventRoute::Thread {
            root_event_id: root.to_string()
        }
    );
    assert_eq!(
        event_route_for_thread_root(root, Some(root.to_owned())),
        NativeTimelineEventRoute::Room
    );
    assert_eq!(
        event_route_for_thread_root(reply, None),
        NativeTimelineEventRoute::Room
    );
}

#[tokio::test]
async fn focused_open_of_a_thread_reply_keeps_the_room_live_and_reports_the_thread() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!focused-thread:example.org");
    let root_id = event_id!("$focused-thread-root");
    let reply_id = event_id!("$focused-thread-reply");
    let main_id = event_id!("$focused-thread-main");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("thread root").sender(*BOB).event_id(root_id))
                .add_timeline_event(
                    f.event(thread_reply(root_id, "reply 12"))
                        .sender(*BOB)
                        .event_id(reply_id),
                )
                .add_timeline_event(f.text_msg("main message").sender(*BOB).event_id(main_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    // A thread reply must never reach `/context`: that builds the SDK's
    // thread-only permalink window.
    server
        .mock_room_event_context()
        .room(room_id)
        .ok(RoomContextResponseTemplate::new(
            f.event(thread_reply(root_id, "reply 12"))
                .sender(*BOB)
                .event_id(reply_id)
                .into_event(),
        ))
        .expect(0)
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 51);
    assert_eq!(
        owner
            .event_route(room_id.as_str(), reply_id.as_str())
            .await
            .unwrap(),
        NativeTimelineEventRoute::Thread {
            root_event_id: root_id.to_string()
        }
    );
    assert_eq!(
        owner
            .event_route(room_id.as_str(), root_id.as_str())
            .await
            .unwrap(),
        NativeTimelineEventRoute::Room
    );

    let opened = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: reply_id.to_string(),
            },
        }),
    )
    .await
    .expect("focused open should finish")
    .expect("a thread-reply route opens the room");
    assert_eq!(
        opened.focus_route,
        Some(NativeTimelineEventRoute::Thread {
            root_event_id: root_id.to_string()
        })
    );
    assert_eq!(
        opened.position,
        TimelineViewPosition::Focused {
            target_event_id: root_id.to_string()
        },
        "the room behind the thread sits at the root"
    );
    assert_eq!(
        owner.debug_stream_is_threaded(&opened.stream_id).await,
        Some(false),
        "a thread-only timeline must never be presented as a room position"
    );
    let rows = row_event_ids(&opened.snapshot.rows);
    assert!(rows.contains(&root_id.as_str()), "{rows:?}");
    assert!(rows.contains(&main_id.as_str()), "{rows:?}");
    assert!(!rows.contains(&reply_id.as_str()), "{rows:?}");

    // The room behind the thread is the live provider: it follows sync and
    // can promote to the live bottom.
    let followed = owner
        .follow_live_tail(crate::app::timeline::NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: main_id.to_string(),
        })
        .await
        .expect("a live-provider focus can follow the live tail");
    assert_eq!(followed.position, TimelineViewPosition::LiveBottom);
}

#[tokio::test]
async fn a_thread_filtered_permalink_window_is_never_presented_as_the_room() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!threaded-window:example.org");
    let root_id = event_id!("$threaded-window-root");
    let reply_id = event_id!("$threaded-window-reply");
    let f = EventFactory::new().room(room_id);
    // Only the root is local, and `/event` fails transiently, so Core cannot
    // tell the reply is threaded before `/context` answers.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("thread root").sender(*BOB).event_id(root_id)),
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
        .room(room_id)
        .ok_with_template(ResponseTemplate::new(500))
        .mount()
        .await;
    server
        .mock_room_event_context()
        .room(room_id)
        .ok(RoomContextResponseTemplate::new(
            f.event(thread_reply(root_id, "late reply"))
                .sender(*BOB)
                .event_id(reply_id)
                .into_event(),
        )
        .events_before(vec![f
            .text_msg("thread root")
            .sender(*BOB)
            .event_id(root_id)
            .into_event()])
        .start("threaded-window-prev")
        .end("threaded-window-next"))
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 52);
    assert_eq!(
        owner
            .event_route(room_id.as_str(), reply_id.as_str())
            .await
            .unwrap(),
        NativeTimelineEventRoute::Unreachable
    );
    let opened = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: reply_id.to_string(),
            },
        }),
    )
    .await
    .expect("focused open should finish")
    .expect("the open falls back to the room");
    assert_eq!(
        opened.focus_route,
        Some(NativeTimelineEventRoute::Thread {
            root_event_id: root_id.to_string()
        }),
        "the thread root comes from the SDK's threaded window"
    );
    assert_eq!(
        opened.position,
        TimelineViewPosition::Focused {
            target_event_id: root_id.to_string()
        }
    );
    assert_eq!(
        owner.debug_stream_is_threaded(&opened.stream_id).await,
        Some(false)
    );
    assert!(
        !owner
            .lock()
            .await
            .focused_entries
            .contains_key(&(room_id.to_string(), reply_id.to_string())),
        "the thread-only window is not kept as a room permalink"
    );
    assert!(!row_event_ids(&opened.snapshot.rows).contains(&reply_id.as_str()));
}

#[tokio::test]
async fn a_missing_target_opens_the_live_room_and_says_so() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!missing-target:example.org");
    let live_id = event_id!("$missing-target-live");
    let gone_id = event_id!("$missing-target-gone");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("still here").sender(*BOB).event_id(live_id)),
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
        .room(room_id)
        .ok_with_template(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "errcode": "M_NOT_FOUND",
            "error": "Event not found.",
        })))
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 53);
    let opened = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: gone_id.to_string(),
            },
        }),
    )
    .await
    .expect("focused open should finish")
    .expect("a missing target still opens the room");
    assert_eq!(opened.focus_route, Some(NativeTimelineEventRoute::Missing));
    assert_eq!(opened.position, TimelineViewPosition::LiveBottom);
    assert!(row_event_ids(&opened.snapshot.rows).contains(&live_id.as_str()));
}

#[tokio::test]
async fn a_sparse_route_finds_the_room_and_thread_of_a_cached_event() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let other_room_id = room_id!("!sparse-other:example.org");
    let room_id = room_id!("!sparse-route:example.org");
    let root_id = event_id!("$sparse-route-root");
    let reply_id = event_id!("$sparse-route-reply");
    let other = EventFactory::new().room(other_room_id);
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(other_room_id)
                .add_state_event(other.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(other.text_msg("elsewhere").sender(*BOB)),
        )
        .await;
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("thread root").sender(*BOB).event_id(root_id))
                .add_timeline_event(
                    f.event(thread_reply(root_id, "sparse reply"))
                        .sender(*BOB)
                        .event_id(reply_id),
                ),
        )
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 54);
    assert_eq!(
        owner.event_location(reply_id.as_str()).await.unwrap(),
        Some(NativeTimelineEventLocation {
            room_id: room_id.to_string(),
            route: NativeTimelineEventRoute::Thread {
                root_event_id: root_id.to_string()
            },
        })
    );
    assert_eq!(
        owner.event_location(root_id.as_str()).await.unwrap(),
        Some(NativeTimelineEventLocation {
            room_id: room_id.to_string(),
            route: NativeTimelineEventRoute::Room,
        })
    );
    assert_eq!(
        owner.event_location("$sparse-route-unknown").await.unwrap(),
        None,
        "an event no room has cached is not guessed"
    );
    assert!(owner.event_location("not-an-event-id").await.is_err());
}
