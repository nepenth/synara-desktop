//! The registry lock is never held across network I/O. A slow `/messages`
//! in one room must not stall media handles, snapshots or other rooms, a slow
//! `/context` must not hold the registry and is bounded, and concurrent opens
//! of one room must share one live timeline. Also covers the live pagination
//! latch following the SDK's un-latch.

use super::*;
use crate::app::timeline::TimelineViewRow;
use matrix_sdk::ruma::{mxc_uri, room_id, OwnedRoomId, RoomVersionId};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::event_id;
use serde_json::json;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

/// Longer than any assertion window below, so a request still in flight
/// proves the lock was free while it waited.
const SLOW: Duration = Duration::from_secs(3);
const PROMPT: Duration = Duration::from_millis(750);

async fn requests_to(server: &MatrixMockServer, suffix: &str) -> usize {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().contains(suffix))
        .count()
}

async fn until_requested(server: &MatrixMockServer, suffix: &str) {
    timeout(Duration::from_secs(5), async {
        while requests_to(server, suffix).await == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("expected a {suffix} request"));
}

/// A room with enough cached events that opening it needs no network page.
async fn sync_cached_room(server: &MatrixMockServer, client: &Client, id: &RoomId) {
    let f = EventFactory::new().room(id);
    server
        .sync_room(
            client,
            JoinedRoomBuilder::new(id)
                .set_timeline_prev_batch("earlier")
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(
                    f.image("cat.png".to_owned(), mxc_uri!("mxc://example.org/cat").to_owned())
                        .sender(*BOB),
                )
                .add_timeline_event(f.text_msg("one").sender(*BOB))
                .add_timeline_event(f.text_msg("two").sender(*BOB)),
        )
        .await;
}

fn live_bottom(room_id: &RoomId) -> NativeTimelineOpenRequest {
    NativeTimelineOpenRequest {
        room_id: room_id.to_string(),
        position: NativeTimelineOpenPosition::LiveBottom,
    }
}

#[tokio::test]
async fn a_slow_back_page_does_not_block_media_snapshots_or_another_room() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let paging_room = room_id!("!slow-page:example.org");
    let other_room = room_id!("!other-room:example.org");
    sync_cached_room(&server, &client, paging_room).await;
    sync_cached_room(&server, &client, other_room).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().with_delay(SLOW))
        .mount()
        .await;

    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 61));
    let opened = owner.open_at(live_bottom(paging_room)).await.unwrap();
    let handle = opened
        .snapshot
        .rows
        .iter()
        .find_map(|row| match row {
            TimelineViewRow::Message(row) => row.media.as_ref(),
            _ => None,
        })
        .expect("the image row carries a media handle")
        .handle_id
        .clone();
    assert_eq!(requests_to(&server, "/messages").await, 0);

    let paging = {
        let owner = owner.clone();
        let stream_id = opened.stream_id.clone();
        tokio::spawn(async move {
            owner
                .paginate(NativeTimelineViewPaginationRequest {
                    stream_id,
                    direction: NativeTimelineDirection::Backwards,
                })
                .await
        })
    };
    until_requested(&server, "/messages").await;

    let source = timeout(PROMPT, owner.resolve_media(&handle))
        .await
        .expect("media resolution must not wait for the page");
    assert!(source.is_some());
    timeout(PROMPT, owner.snapshot(&opened.stream_id))
        .await
        .expect("a snapshot must not wait for the page")
        .unwrap();
    let other = timeout(PROMPT, owner.open_at(live_bottom(other_room)))
        .await
        .expect("opening another room must not wait for the page")
        .unwrap();
    timeout(PROMPT, owner.snapshot(&other.stream_id))
        .await
        .expect("another room's snapshot must not wait for the page")
        .unwrap();
    assert!(!paging.is_finished(), "the page is still in flight");

    let paged = timeout(Duration::from_secs(10), paging)
        .await
        .expect("the page finishes")
        .unwrap()
        .unwrap();
    assert_eq!(paged.room_id, paging_room.as_str());
}

#[tokio::test]
async fn a_slow_context_build_holds_no_registry_lock_and_is_bounded() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let focused_room = room_id!("!slow-context:example.org");
    let other_room = room_id!("!other-context:example.org");
    let target = event_id!("$older-than-the-live-window");
    sync_cached_room(&server, &client, focused_room).await;
    sync_cached_room(&server, &client, other_room).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    // The target exists in the room but outside the live window, so the
    // open builds an event-focused timeline through `/context`.
    server
        .mock_room_event()
        .room(focused_room)
        .ok(EventFactory::new()
            .room(focused_room)
            .text_msg("older")
            .sender(*BOB)
            .event_id(target)
            .into_event())
        .mount()
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/context/"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({"errcode": "M_NOT_FOUND", "error": "gone"}))
                .set_delay(Duration::from_secs(60)),
        )
        .mount(server.server())
        .await;

    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 62));
    // matrix-sdk 0.19 holds its event-cache-wide state lock while it fetches
    // `/context`, so anything reading the SDK event cache (timeline builds,
    // snapshots, read state) waits in the SDK until that request ends. Core's
    // own registry and media handles must not wait.
    let earlier = owner.open_at(live_bottom(other_room)).await.unwrap();
    let handle = earlier
        .snapshot
        .rows
        .iter()
        .find_map(|row| match row {
            TimelineViewRow::Message(row) => row.media.as_ref(),
            _ => None,
        })
        .expect("the image row carries a media handle")
        .handle_id
        .clone();
    let focusing = {
        let owner = owner.clone();
        tokio::spawn(async move {
            owner
                .open_at(NativeTimelineOpenRequest {
                    room_id: focused_room.to_string(),
                    position: NativeTimelineOpenPosition::Focused {
                        event_id: target.to_string(),
                    },
                })
                .await
        })
    };
    until_requested(&server, "/context/").await;

    drop(
        timeout(PROMPT, owner.lock())
            .await
            .expect("the registry lock is free while /context is in flight"),
    );
    let source = timeout(PROMPT, owner.resolve_media(&handle))
        .await
        .expect("media resolution must not wait for /context");
    assert!(source.is_some());
    let closed = timeout(PROMPT, async {
        let mut registry = owner.lock().await;
        registry.close_view(NativeTimelineCloseRequest {
            stream_id: earlier.stream_id.clone(),
        })
    })
    .await
    .expect("closing a stream must not wait for /context");
    assert!(closed);

    // Snapshots read the SDK event cache, which waits for the SDK lock, so
    // Core bounds the build instead of letting the request run its retries.
    let focused = timeout(FOCUSED_BUILD_TIMEOUT + Duration::from_secs(3), focusing)
        .await
        .expect("an event-focused build is bounded")
        .unwrap()
        .expect("a slow /context keeps the room usable");
    assert_eq!(focused.position, TimelineViewPosition::LiveBottom);
    assert_eq!(
        focused.focus_route,
        Some(NativeTimelineEventRoute::Unreachable)
    );
    // Giving up released the SDK lock: other rooms answer at once again.
    let reopened = timeout(PROMPT, owner.open_at(live_bottom(other_room)))
        .await
        .expect("other rooms open once the bounded build gives up")
        .unwrap();
    timeout(PROMPT, owner.snapshot(&reopened.stream_id))
        .await
        .expect("snapshots answer once the bounded build gives up")
        .unwrap();
}

#[tokio::test]
async fn concurrent_opens_of_one_room_share_one_live_timeline() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!opened-twice:example.org");
    let f = EventFactory::new().room(id);
    // A sparse cache, so building the live timeline pages the network.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .set_timeline_prev_batch("earlier")
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("sparse cached tail").sender(*BOB)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().with_delay(Duration::from_millis(500)))
        .mount()
        .await;

    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 63));
    let open = |owner: Arc<NativeTimelineOwner>| {
        tokio::spawn(async move { owner.open_at(live_bottom(id)).await })
    };
    let (first, second) = (open(owner.clone()), open(owner.clone()));
    until_requested(&server, "/messages").await;
    drop(
        timeout(PROMPT, owner.lock())
            .await
            .expect("the registry lock is free while the live timeline builds"),
    );
    let first = first.await.unwrap().unwrap();
    let second = second.await.unwrap().unwrap();
    assert_ne!(first.stream_id, second.stream_id);

    let registry = owner.lock().await;
    assert_eq!(registry.entries.len(), 1, "one live entry for the room");
    let live = &registry.entries[id.as_str()].timeline;
    for stream_id in [&first.stream_id, &second.stream_id] {
        assert!(
            Arc::ptr_eq(live, &registry.view_streams[stream_id].timeline),
            "every stream of the room uses the installed live timeline"
        );
    }
    drop(registry);
    assert_eq!(
        requests_to(&server, "/messages").await,
        1,
        "the second open waits for the first build instead of starting another"
    );
}

#[tokio::test]
async fn the_live_pagination_latch_follows_the_sdk_after_a_gappy_sync() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id: OwnedRoomId = room_id!("!gappy:example.org").to_owned();
    sync_cached_room(&server, &client, &id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // No `end` token: the first page reaches the start of the room.
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;

    let batches = Arc::new(Mutex::new(Vec::<TimelineViewDeltaBatch>::new()));
    let emit: TimelineViewUpdateEmit = {
        let batches = batches.clone();
        Arc::new(move |batch| batches.lock().unwrap().push(batch))
    };
    let owner = NativeTimelineOwner::new(&client, emit, 64);
    let opened = owner.open_at(live_bottom(&id)).await.unwrap();
    assert_eq!(
        opened.snapshot.pagination.backward,
        TimelinePageState::Available
    );
    let mut backward = TimelinePageState::Available;
    for _ in 0..3 {
        backward = owner
            .paginate(NativeTimelineViewPaginationRequest {
                stream_id: opened.stream_id.clone(),
                direction: NativeTimelineDirection::Backwards,
            })
            .await
            .unwrap()
            .pagination
            .backward;
        if backward == TimelinePageState::Exhausted {
            break;
        }
    }
    assert_eq!(backward, TimelinePageState::Exhausted);
    // A new stream is seeded from the SDK's current status.
    let seeded = owner.open_at(live_bottom(&id)).await.unwrap();
    assert_eq!(
        seeded.snapshot.pagination.backward,
        TimelinePageState::Exhausted
    );

    // A limited sync with a gap: the SDK keeps only the newest chunk and
    // reports that the start is no longer loaded.
    batches.lock().unwrap().clear();
    let f = EventFactory::new().room(&id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(&id)
                .set_timeline_limited()
                .set_timeline_prev_batch("gap")
                .add_timeline_event(f.text_msg("after the gap").sender(*BOB)),
        )
        .await;
    timeout(Duration::from_secs(5), async {
        loop {
            let unlatched = batches.lock().unwrap().iter().any(|batch| {
                batch.stream_id == opened.stream_id
                    && batch
                        .pagination
                        .as_ref()
                        .is_some_and(|state| state.backward == TimelinePageState::Available)
            });
            if unlatched {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the stream reports earlier history as available again");
    for stream_id in [&opened.stream_id, &seeded.stream_id] {
        assert_eq!(
            owner.snapshot(stream_id).await.unwrap().pagination.backward,
            TimelinePageState::Available
        );
    }
    let reopened = owner.open_at(live_bottom(&id)).await.unwrap();
    assert_eq!(
        reopened.snapshot.pagination.backward,
        TimelinePageState::Available,
        "new streams are not seeded from a stale per-room latch"
    );
}
