//! Cached SDK windows and side paths around the view streams: readbacks
//! never build a smaller permalink window or one for a loaded row, idle
//! permalink windows are dropped, reply parents outside the window are
//! fetched, pin-panel media resolves, thread-list counts are bounded, a
//! room not yet synced is waited for, Jump to latest keeps a live stream,
//! the safety-net probe copies no rows, and resume drops stale caches.

use super::published::ViewChange;
use super::*;
use crate::app::timeline::{TimelineViewDeltaOp, TimelineViewRow};
use matrix_sdk::ruma::{event_id, mxc_uri, room_id, OwnedEventId, RoomVersionId};
use matrix_sdk::test_utils::mocks::{
    MatrixMockServer, RoomContextResponseTemplate, RoomMessagesResponseTemplate,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};

const WAIT: Duration = Duration::from_secs(8);

fn quiet_owner(client: &Client, generation: u64) -> NativeTimelineOwner {
    NativeTimelineOwner::new(client, Arc::new(|_| {}), generation)
}

fn open_request(
    room_id: &RoomId,
    position: NativeTimelineOpenPosition,
) -> NativeTimelineOpenRequest {
    NativeTimelineOpenRequest {
        room_id: room_id.to_string(),
        position,
    }
}

fn message_ids(rows: &[TimelineViewRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            TimelineViewRow::Message(row) => row.event.event_id.clone(),
            _ => None,
        })
        .collect()
}

fn message_event_id(index: usize) -> OwnedEventId {
    OwnedEventId::try_from(format!("$cache-message-{index}")).unwrap()
}

/// A plain room whose create event and `count` messages are cached.
async fn cached_room(server: &MatrixMockServer, client: &Client, room_id: &RoomId, count: usize) {
    let f = EventFactory::new().room(room_id);
    let mut room = JoinedRoomBuilder::new(room_id)
        .set_timeline_prev_batch("earlier")
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11));
    for index in 0..count {
        room = room.add_timeline_event(
            f.text_msg(format!("message {index}"))
                .sender(*BOB)
                .event_id(&message_event_id(index)),
        );
    }
    server.sync_room(client, room).await;
}

async fn mount_quiet_history(server: &MatrixMockServer) {
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
}

/// Requests the mock homeserver received whose path contains `segment`.
async fn requests_to(server: &MatrixMockServer, segment: &str) -> Vec<wiremock::Request> {
    server
        .server()
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|request| request.url.path().contains(segment))
        .collect()
}

#[tokio::test]
async fn readback_of_a_loaded_row_outside_the_skip_window_builds_no_permalink_window() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-readback:example.org");
    cached_room(&server, &client, room_id, 30).await;
    mount_quiet_history(&server).await;
    let owner = quiet_owner(&client, 61);
    let opened = timeout(
        WAIT,
        owner.open_at(open_request(
            room_id,
            NativeTimelineOpenPosition::LiveBottom,
        )),
    )
    .await
    .unwrap()
    .unwrap();
    let oldest = message_event_id(1);
    assert!(
        !message_ids(&opened.snapshot.rows).contains(&oldest.to_string()),
        "the view's skip window starts after the oldest rows"
    );

    let readback = timeout(
        WAIT,
        owner.event_readback(room_id.as_str(), oldest.as_str()),
    )
    .await
    .unwrap()
    .expect("a loaded row reads back from the live provider");
    assert_eq!(readback.item.event_id, oldest.as_str());
    // A reaction readback of the same old row also stays on the live
    // provider (it used to search only the view's skip window).
    let reaction = timeout(
        WAIT,
        reaction_readback(
            &mut Slots::Shared(&owner),
            &client,
            room_id.as_str(),
            &oldest,
            "👍",
            false,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(reaction.is_none());
    assert!(requests_to(&server, "/context/").await.is_empty());
    assert!(owner.lock().await.focused_entries.iter().next().is_none());
}

#[tokio::test]
async fn readbacks_build_the_one_permalink_window_and_closing_its_stream_drops_it() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-permalink:example.org");
    let target = event_id!("$cache-permalink-target");
    let before = event_id!("$cache-permalink-before");
    let after = event_id!("$cache-permalink-after");
    cached_room(&server, &client, room_id, 3).await;
    mount_quiet_history(&server).await;
    let f = EventFactory::new().room(room_id);
    server
        .mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("target")
            .sender(*BOB)
            .event_id(target)
            .into_event())
        .mount()
        .await;
    server
        .mock_room_event_context()
        .room(room_id)
        .ok(RoomContextResponseTemplate::new(
            f.text_msg("target")
                .sender(*BOB)
                .event_id(target)
                .into_event(),
        )
        .events_before(vec![f
            .text_msg("before")
            .sender(*BOB)
            .event_id(before)
            .into_event()])
        .events_after(vec![f
            .text_msg("after")
            .sender(*BOB)
            .event_id(after)
            .into_event()])
        .start("cache-permalink-prev")
        .end("cache-permalink-next"))
        .mount()
        .await;
    let owner = quiet_owner(&client, 62);

    // An approval or notification readback of an unloaded event comes first.
    let readback = timeout(
        WAIT,
        owner.event_readback(room_id.as_str(), target.as_str()),
    )
    .await
    .unwrap()
    .expect("an unloaded event reads back from its permalink window");
    assert_eq!(readback.item.event_id, target.as_str());
    let contexts = requests_to(&server, "/context/").await;
    assert_eq!(contexts.len(), 1);
    let limit = contexts[0]
        .url
        .query_pairs()
        .find(|(key, _)| key == "limit")
        .map(|(_, value)| value.into_owned());
    assert_eq!(
        limit.as_deref(),
        Some(FOCUSED_CONTEXT_EVENT_COUNT.to_string().as_str()),
        "readbacks must not seed the SDK's per-event memo with a smaller window"
    );

    // The permalink open reuses that window with its context around it.
    let opened = timeout(
        WAIT,
        owner.open_at(open_request(
            room_id,
            NativeTimelineOpenPosition::Focused {
                event_id: target.to_string(),
            },
        )),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        opened.position,
        TimelineViewPosition::Focused {
            target_event_id: target.to_string()
        }
    );
    let rows = message_ids(&opened.snapshot.rows);
    assert!(rows.contains(&before.to_string()), "{rows:?}");
    assert!(rows.contains(&after.to_string()), "{rows:?}");
    assert_eq!(requests_to(&server, "/context/").await.len(), 1);

    let key = (room_id.to_string(), target.to_string());
    assert!(owner.lock().await.focused_entries.contains_key(&key));
    assert!(owner.lock().await.close_view(NativeTimelineCloseRequest {
        stream_id: opened.stream_id,
    }));
    assert!(
        !owner.lock().await.focused_entries.contains_key(&key),
        "an idle permalink window is dropped with its last stream"
    );
}

#[tokio::test]
async fn closing_a_stream_lets_its_reply_parent_fetches_finish() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-reply-cancel:example.org");
    let parent = event_id!("$cache-reply-cancel-parent");
    let reply = event_id!("$cache-reply-cancel");
    let f = EventFactory::new().room(room_id);
    let room = server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("first").sender(*BOB))
                .add_timeline_event(
                    f.text_msg("answering an old message")
                        .reply_to(parent)
                        .sender(*BOB)
                        .event_id(reply),
                ),
        )
        .await;
    mount_quiet_history(&server).await;
    let parent_event = f
        .text_msg("the old question")
        .sender(*BOB)
        .event_id(parent)
        .into_raw_timeline();
    server
        .mock_room_event()
        .match_event_id()
        .ok_with_template(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(parent_event.json())
                .set_delay(Duration::from_millis(300)),
        )
        .mount()
        .await;
    let timeline = Arc::new(
        TimelineBuilder::new(&room)
            .with_focus(TimelineFocus::Live {
                hide_threaded_events: HIDE_THREADED_EVENTS,
            })
            .build()
            .await
            .unwrap(),
    );
    let reply_details = |timeline: &Arc<Timeline>| {
        let timeline = timeline.clone();
        async move {
            timeline
                .item_by_event_id(reply)
                .await
                .and_then(|item| item.content().in_reply_to())
                .map(|details| details.event)
        }
    };

    let mut fetches = ReplyDetailFetches::new(timeline.clone());
    fetches.request_items(timeline.items().await.iter());
    assert_eq!(
        fetches.in_flight(),
        1,
        "the reply's parent is being fetched"
    );
    // Close the stream while its fetch waits for the homeserver.
    timeout(WAIT, async {
        while requests_to(&server, parent.as_str()).await.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        reply_details(&timeline).await,
        Some(TimelineDetails::Pending)
    ));
    drop(fetches);
    // The detached fetch still completes. Cancelling it would leave the
    // SDK's `Pending` reply, which the SDK never fetches again.
    timeout(WAIT, async {
        while !matches!(
            reply_details(&timeline).await,
            Some(TimelineDetails::Ready(_))
        ) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the reply's parent arrives after its stream closed");
    assert_eq!(requests_to(&server, parent.as_str()).await.len(), 1);
}

#[tokio::test]
async fn a_reply_to_a_parent_outside_the_window_resolves_its_preview() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-reply:example.org");
    let parent = event_id!("$cache-reply-parent");
    let reply = event_id!("$cache-reply");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .set_timeline_prev_batch("earlier")
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("first").sender(*BOB))
                .add_timeline_event(
                    f.text_msg("answering an old message")
                        .reply_to(parent)
                        .sender(*BOB)
                        .event_id(reply),
                ),
        )
        .await;
    mount_quiet_history(&server).await;
    server
        .mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("the old question")
            .sender(*BOB)
            .event_id(parent)
            .into_event())
        .expect(1)
        .mount()
        .await;
    let owner = quiet_owner(&client, 63);
    let opened = timeout(
        WAIT,
        owner.open_at(open_request(
            room_id,
            NativeTimelineOpenPosition::LiveBottom,
        )),
    )
    .await
    .unwrap()
    .unwrap();

    let preview = timeout(WAIT, async {
        loop {
            let snapshot = owner.snapshot(&opened.stream_id).await.unwrap();
            let preview = snapshot.rows.iter().find_map(|row| match row {
                TimelineViewRow::Message(row)
                    if row.event.event_id.as_deref() == Some(reply.as_str()) =>
                {
                    row.reply.clone()
                }
                _ => None,
            });
            if let Some(preview) = preview.filter(|preview| preview.sender_id.is_some()) {
                break preview;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the reply's parent is fetched and the row updated");
    assert_eq!(preview.event_id, parent.as_str());
    assert_eq!(preview.body, "the old question");
}

#[tokio::test]
async fn pin_panel_media_resolves_and_a_refresh_sends_no_relations_requests() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-pins:example.org");
    let pinned = event_id!("$cache-pinned-image");
    let f = EventFactory::new().room(room_id).sender(*BOB);
    let image = || {
        f.image(
            "cat.png".to_owned(),
            mxc_uri!("mxc://example.org/cat").to_owned(),
        )
        .sender(*BOB)
        .event_id(pinned)
    };
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_state_bulk(vec![f.room_pinned_events(vec![pinned.to_owned()]).into()])
                .add_timeline_event(image()),
        )
        .await;
    mount_quiet_history(&server).await;
    server
        .mock_room_event()
        .match_event_id()
        .ok(image().into_event())
        .mount()
        .await;
    let owner = quiet_owner(&client, 64);

    let first = timeout(WAIT, async {
        loop {
            let snapshot = owner
                .pinned_events_snapshot(room_id.as_str())
                .await
                .unwrap();
            if snapshot.items.iter().any(|item| item.media.is_some()) {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the pinned image is projected with a media handle");
    let handle = first.items[0].media.as_ref().unwrap().handle_id.clone();
    let source = owner
        .resolve_media(&handle)
        .await
        .expect("a pin-panel handle resolves without any open view stream");
    assert_eq!(source.item_id, pinned.as_str());

    let relations_before = requests_to(&server, "/relations/").await.len();
    let second = owner
        .pinned_events_snapshot(room_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        second.items[0].media.as_ref().unwrap().handle_id,
        handle,
        "a refreshed pin keeps its handle"
    );
    assert_eq!(
        requests_to(&server, "/relations/").await.len(),
        relations_before,
        "a pin-panel refresh fetches no relations"
    );
    assert!(owner.resolve_media(&handle).await.is_some());
}

#[tokio::test]
async fn thread_list_counts_only_the_newest_threads() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-threads:example.org");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11)),
        )
        .await;
    let total = MAX_THREAD_UNREAD_COUNTS + 8;
    let roots = (0..total)
        .map(|index| {
            f.text_msg(format!("root {index}"))
                .sender(*BOB)
                .event_id(&OwnedEventId::try_from(format!("$cache-root-{index}")).unwrap())
                .server_ts(1_700_000_000_000_u64 + index as u64)
                .into_raw()
        })
        .collect();
    server.mock_room_threads().ok(roots, None).mount().await;
    let owner = quiet_owner(&client, 65);

    let snapshot = timeout(WAIT, owner.thread_list(room_id.as_str(), "open"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.threads.len(), total);
    let counted = snapshot
        .threads
        .iter()
        .filter(|thread| thread.unread_count.is_some())
        .count();
    assert_eq!(counted, MAX_THREAD_UNREAD_COUNTS);
    assert!(
        snapshot.threads[..MAX_THREAD_UNREAD_COUNTS]
            .iter()
            .all(|thread| thread.unread_count.is_some()),
        "the newest threads are the counted ones"
    );
}

#[tokio::test]
async fn thread_list_services_are_bounded_per_room() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let f = EventFactory::new();
    let rooms: Vec<OwnedRoomId> = (0..=MAX_THREAD_LIST_ROOMS)
        .map(|index| OwnedRoomId::try_from(format!("!cache-list-{index}:example.org")).unwrap())
        .collect();
    for room_id in &rooms {
        server
            .sync_room(
                &client,
                JoinedRoomBuilder::new(room_id)
                    .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11)),
            )
            .await;
    }
    server
        .mock_room_threads()
        .ok(Vec::new(), None)
        .mount()
        .await;
    let owner = quiet_owner(&client, 66);
    for room_id in &rooms {
        timeout(WAIT, owner.thread_list(room_id.as_str(), "open"))
            .await
            .unwrap()
            .unwrap();
    }
    let lists = owner.thread_lists.lock().await;
    assert_eq!(lists.len(), MAX_THREAD_LIST_ROOMS);
    assert!(
        !lists.contains_key(rooms[0].as_str()),
        "the least recently listed room's service is dropped"
    );
}

#[tokio::test]
async fn an_open_waits_for_sync_to_bring_in_an_unknown_room() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let owner = quiet_owner(&client, 67);
    let missing = room_id!("!cache-never:example.org");
    assert_eq!(
        owner
            .await_room(missing.as_str(), Duration::from_millis(50))
            .await,
        Err("v-timeline-room-not-yet-synced")
    );
    assert_eq!(
        owner
            .await_room("not-a-room", Duration::from_millis(50))
            .await,
        Err("d0.3-timeline-invalid-room-id")
    );

    let joined = room_id!("!cache-joined-elsewhere:example.org");
    let wait = owner.await_room(joined.as_str(), WAIT);
    let sync = async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cached_room(&server, &client, joined, 1).await;
    };
    let (waited, ()) = tokio::join!(wait, sync);
    assert_eq!(waited, Ok(()));
    // A known room never waits.
    assert_eq!(
        owner.await_room(joined.as_str(), Duration::ZERO).await,
        Ok(())
    );
}

#[tokio::test]
async fn jump_to_latest_keeps_a_live_stream_and_the_probe_copies_no_rows() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!cache-jump:example.org");
    cached_room(&server, &client, room_id, 3).await;
    mount_quiet_history(&server).await;
    let owner = quiet_owner(&client, 68);
    let opened = timeout(
        WAIT,
        owner.open_at(open_request(
            room_id,
            NativeTimelineOpenPosition::LiveBottom,
        )),
    )
    .await
    .unwrap()
    .unwrap();

    let jumped = owner
        .jump_latest(NativeTimelineJumpLatestRequest {
            stream_id: opened.stream_id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        jumped.stream_id, opened.stream_id,
        "no reopen when already live"
    );
    assert_eq!(jumped.position, TimelineViewPosition::LiveBottom);
    assert_eq!(owner.lock().await.view_streams.len(), 1);

    let revision = owner.snapshot(&opened.stream_id).await.unwrap().revision;
    assert!(owner
        .snapshot_after(&opened.stream_id, revision)
        .await
        .unwrap()
        .is_none());
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(f.text_msg("newer").sender(*BOB)),
        )
        .await;
    let newer = timeout(WAIT, async {
        loop {
            if let Some(snapshot) = owner
                .snapshot_after(&opened.stream_id, revision)
                .await
                .unwrap()
            {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(newer.revision > revision);
}

#[tokio::test]
async fn resume_drops_idle_windows_and_restarts_the_unclaimed_clock() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let shown = room_id!("!cache-resume-shown:example.org");
    let idle = room_id!("!cache-resume-idle:example.org");
    cached_room(&server, &client, shown, 2).await;
    cached_room(&server, &client, idle, 2).await;
    mount_quiet_history(&server).await;
    let owner = quiet_owner(&client, 69).with_unclaimed_stream_ttl(Duration::ZERO);
    let opened = timeout(
        WAIT,
        owner.open_at(open_request(shown, NativeTimelineOpenPosition::LiveBottom)),
    )
    .await
    .unwrap()
    .unwrap();
    // Room-list Mark as Read leaves an idle live timeline behind.
    open_room_read_timeline_in(&mut Slots::Shared(&owner), &client, idle.as_str())
        .await
        .unwrap();
    assert!(owner.lock().await.entries.contains_key(idle.as_str()));

    let view = owner
        .lock()
        .await
        .view_streams
        .get(&opened.stream_id)
        .unwrap()
        .view
        .clone();
    let clear = || ViewChange {
        ops: vec![TimelineViewDeltaOp::Clear],
        ..ViewChange::default()
    };
    // One batch left unread while suspended starts the clock...
    view.claim();
    assert!(view.publish(clear()));
    owner.resumed().await;
    // ...and resume restarts it, so the first batch after resume does not
    // expire the stream its consumer is about to read.
    assert!(view.publish(clear()));
    assert!(!view.is_expired());

    let registry = owner.lock().await;
    assert!(registry.entries.contains_key(shown.as_str()));
    assert!(
        !registry.entries.contains_key(idle.as_str()),
        "a live timeline no stream shows is rebuilt after resume"
    );
}
