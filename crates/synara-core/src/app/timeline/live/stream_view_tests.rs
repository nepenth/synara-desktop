//! A view stream's published rows, revision and metadata against a mocked
//! homeserver: snapshots and deltas agree at every revision, an authority
//! reset re-projects only the streamed window, receipt-only replacements
//! cost nothing, forward pagination is reported per focus, Jump to latest
//! and Follow live keep the stream consistent, unread streams expire, and
//! legacy UTD bookkeeping never fails an open.

use super::published::apply_view_ops;
use super::*;
use crate::app::timeline::{TimelineViewDeltaOp, TimelineViewRow, MAX_UTD_ENTRIES};
use matrix_sdk::ruma::events::receipt::ReceiptThread;
use matrix_sdk::ruma::events::room::encrypted::{
    EncryptedEventScheme, MegolmV1AesSha2ContentInit, RoomEncryptedEventContent,
};
use matrix_sdk::ruma::events::AnyRoomAccountDataEvent;
use matrix_sdk::ruma::serde::Raw;
use matrix_sdk::ruma::{event_id, room_id, OwnedEventId, RoomVersionId};
use matrix_sdk::test_utils::mocks::{
    MatrixMockServer, RoomContextResponseTemplate, RoomMessagesResponseTemplate,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB, CAROL};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

const WAIT: Duration = Duration::from_secs(5);

type Batches = UnboundedReceiver<TimelineViewDeltaBatch>;

fn owner_with_batches(client: &Client, generation: u64) -> (NativeTimelineOwner, Batches) {
    let (sender, batches) = unbounded_channel();
    let owner = NativeTimelineOwner::new(
        client,
        Arc::new(move |batch| {
            let _ = sender.send(batch);
        }),
        generation,
    );
    (owner, batches)
}

fn live_bottom(room_id: &RoomId) -> NativeTimelineOpenRequest {
    NativeTimelineOpenRequest {
        room_id: room_id.to_string(),
        position: NativeTimelineOpenPosition::LiveBottom,
    }
}

fn row_item_id(row: &TimelineViewRow) -> &str {
    match row {
        TimelineViewRow::Message(row) => &row.event.item_id,
        TimelineViewRow::Poll(row) => &row.event.item_id,
        TimelineViewRow::Sticker { event, .. } => &event.item_id,
        TimelineViewRow::Membership(row) => &row.event.item_id,
        TimelineViewRow::State(row) => &row.event.item_id,
        TimelineViewRow::Call(row) => &row.event.item_id,
        TimelineViewRow::Redacted(row) => &row.event.item_id,
        TimelineViewRow::EncryptedUnavailable(row) => &row.event.item_id,
        TimelineViewRow::Other(row) => &row.item_id,
        TimelineViewRow::DateSeparator { item_id, .. }
        | TimelineViewRow::ReadMarker { item_id }
        | TimelineViewRow::UnreadMarker { item_id }
        | TimelineViewRow::TimelineStart { item_id }
        | TimelineViewRow::Pagination { item_id, .. } => item_id,
    }
}

fn message_event_ids(rows: &[TimelineViewRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            TimelineViewRow::Message(row) => row.event.event_id.clone(),
            _ => None,
        })
        .collect()
}

async fn sync_messages(
    server: &MatrixMockServer,
    client: &Client,
    room_id: &RoomId,
    bodies: impl IntoIterator<Item = String>,
) {
    let f = EventFactory::new().room(room_id);
    let mut room = JoinedRoomBuilder::new(room_id);
    for body in bodies {
        room = room.add_timeline_event(f.text_msg(body).sender(*BOB));
    }
    server.sync_room(client, room).await;
}

/// A room whose create event and `count` messages are cached.
async fn cached_room(server: &MatrixMockServer, client: &Client, room_id: &RoomId, count: usize) {
    let f = EventFactory::new().room(room_id);
    let mut room = JoinedRoomBuilder::new(room_id)
        .set_timeline_prev_batch("earlier")
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11));
    for index in 0..count {
        room = room.add_timeline_event(
            f.text_msg(format!("message {index}"))
                .sender(*BOB)
                .event_id(&OwnedEventId::try_from(format!("$message-{index}")).unwrap()),
        );
    }
    server.sync_room(client, room).await;
    server.mock_room_state_encryption().plain().mount().await;
}

async fn until_snapshot(
    owner: &NativeTimelineOwner,
    stream_id: &str,
    mut done: impl FnMut(&TimelineViewSnapshot) -> bool,
) -> TimelineViewSnapshot {
    timeout(WAIT, async {
        loop {
            let snapshot = owner.snapshot(stream_id).await.unwrap();
            if done(&snapshot) {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the stream reached the expected state")
}

#[tokio::test]
async fn snapshots_taken_mid_stream_replay_every_later_batch_exactly_once() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!snapshot-atomicity:example.org");
    cached_room(&server, &client, room_id, 3).await;
    let (owner, mut batches) = owner_with_batches(&client, 71);
    let owner = Arc::new(owner);
    let opened = owner.open_at(live_bottom(room_id)).await.unwrap();

    // Snapshot continuously while sync delivers several batches, so some
    // snapshots land while the update task is between an SDK diff and its
    // emit.
    let sampling = {
        let owner = owner.clone();
        let stream_id = opened.stream_id.clone();
        tokio::spawn(async move {
            let mut snapshots = Vec::new();
            for _ in 0..400 {
                snapshots.push(owner.snapshot(&stream_id).await.unwrap());
                tokio::task::yield_now().await;
            }
            snapshots
        })
    };
    for round in 0..8 {
        sync_messages(
            &server,
            &client,
            room_id,
            (0..3).map(|index| format!("round {round} message {index}")),
        )
        .await;
    }
    let final_snapshot = until_snapshot(&owner, &opened.stream_id, |snapshot| {
        message_event_ids(&snapshot.rows).len() >= 3 + 24
    })
    .await;
    let mut snapshots = sampling.await.unwrap();
    snapshots.push(opened.snapshot.clone());

    let mut emitted = Vec::new();
    while let Ok(batch) = batches.try_recv() {
        emitted.push(batch);
    }
    assert_eq!(
        emitted.last().map(|batch| batch.revision),
        Some(final_snapshot.revision),
        "the last emitted batch is the final snapshot's revision"
    );
    let item_ids = final_snapshot
        .rows
        .iter()
        .map(row_item_id)
        .collect::<HashSet<_>>();
    assert_eq!(
        item_ids.len(),
        final_snapshot.rows.len(),
        "no duplicate rows"
    );

    let revisions = snapshots
        .iter()
        .map(|snapshot| snapshot.revision)
        .collect::<HashSet<_>>();
    assert!(revisions.len() > 2, "snapshots sampled several revisions");
    for snapshot in &snapshots {
        let mut rows = snapshot.rows.clone();
        for batch in emitted
            .iter()
            .filter(|batch| batch.revision > snapshot.revision)
        {
            apply_view_ops(&mut rows, &batch.ops);
        }
        assert_eq!(
            rows, final_snapshot.rows,
            "a snapshot at revision {} plus every later batch is the final rows",
            snapshot.revision
        );
    }
}

#[tokio::test]
async fn an_authority_reset_carries_only_the_streamed_window() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!authority-window:example.org");
    let own_user_id = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(room_id);
    let mut ordinary = std::collections::BTreeMap::from([(own_user_id.clone(), 0.into())]);
    let mut room = JoinedRoomBuilder::new(room_id)
        .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
        .add_state_event(
            f.power_levels(&mut ordinary)
                .sender(&own_user_id)
                .state_key(""),
        );
    for index in 0..40 {
        room = room.add_timeline_event(f.text_msg(format!("message {index}")).sender(*BOB));
    }
    server.sync_room(&client, room).await;
    server.mock_room_state_encryption().plain().mount().await;

    let (owner, mut batches) = owner_with_batches(&client, 72);
    let opened = owner.open_at(live_bottom(room_id)).await.unwrap();
    let timeline = owner
        .registry
        .lock()
        .await
        .view_streams
        .get(&opened.stream_id)
        .unwrap()
        .timeline
        .clone();
    assert!(
        timeline.items().await.len() > opened.snapshot.rows.len(),
        "the SDK skips older items, or the proof is vacuous"
    );

    let mut moderator = std::collections::BTreeMap::from([(own_user_id.clone(), 100.into())]);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_state_event(
                f.power_levels(&mut moderator)
                    .sender(&own_user_id)
                    .state_key(""),
            ),
        )
        .await;
    let mut presented = opened.snapshot.rows.clone();
    let reset = timeout(WAIT, async {
        loop {
            let batch = batches.recv().await.expect("view update stream ended");
            let is_reset = batch
                .ops
                .iter()
                .any(|op| matches!(op, TimelineViewDeltaOp::Reset { .. }));
            apply_view_ops(&mut presented, &batch.ops);
            if is_reset {
                break batch;
            }
        }
    })
    .await
    .expect("an authority reset");
    let snapshot = owner.snapshot(&opened.stream_id).await.unwrap();
    assert!(snapshot.revision >= reset.revision);
    let streamed = timeline.subscribe().await.0.len();
    let TimelineViewDeltaOp::Reset { rows } = reset
        .ops
        .iter()
        .find(|op| matches!(op, TimelineViewDeltaOp::Reset { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(rows.len(), streamed, "the reset is the streamed window");
    assert!(rows.len() < timeline.items().await.len());
    if snapshot.revision == reset.revision {
        assert_eq!(presented, snapshot.rows);
    }
}

#[tokio::test]
async fn receipt_only_replacements_emit_nothing() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!receipt-noise:example.org");
    cached_room(&server, &client, room_id, 3).await;
    let (owner, mut batches) = owner_with_batches(&client, 73);
    let opened = owner.open_at(live_bottom(room_id)).await.unwrap();
    let opened_revision = owner.snapshot(&opened.stream_id).await.unwrap().revision;
    while batches.try_recv().is_ok() {}

    let f = EventFactory::new().room(room_id);
    let receipts = f
        .read_receipts()
        .add(
            event_id!("$message-1"),
            *CAROL,
            matrix_sdk::ruma::events::receipt::ReceiptType::Read,
            ReceiptThread::Unthreaded,
        )
        .into_event();
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_receipt(receipts),
        )
        .await;
    // A later message is the sync barrier: its batch follows any batch the
    // receipt would have produced.
    sync_messages(&server, &client, room_id, ["after the receipt".to_owned()]).await;
    let arrival = timeout(WAIT, async {
        loop {
            let batch = batches.recv().await.expect("view update stream ended");
            if !batch.ops.is_empty() {
                break batch;
            }
        }
    })
    .await
    .expect("the message batch");
    assert_eq!(
        arrival.revision,
        opened_revision + 1,
        "no batch was emitted for the receipt"
    );
    assert!(arrival.ops.iter().all(|op| !matches!(
        op,
        TimelineViewDeltaOp::Set { index, .. } if *index < opened.snapshot.rows.len()
    )));
}

#[tokio::test]
async fn forward_pagination_is_reported_per_focus() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!forward-focus:example.org");
    cached_room(&server, &client, room_id, 3).await;
    let (owner, _batches) = owner_with_batches(&client, 74);

    // Live: always at the newest event, and paging forward says so.
    let live = owner.open_at(live_bottom(room_id)).await.unwrap();
    assert_eq!(
        live.snapshot.pagination.forward,
        TimelinePageState::Exhausted
    );
    assert!(live.snapshot.capabilities.paginate_forward);
    let paged = owner
        .paginate(NativeTimelineViewPaginationRequest {
            stream_id: live.stream_id.clone(),
            direction: NativeTimelineDirection::Forwards,
        })
        .await
        .unwrap();
    assert_eq!(paged.pagination.forward, TimelinePageState::Exhausted);

    // Thread: no forward pagination, and a forward request is not an error.
    let thread = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Thread {
                root_event_id: "$message-0".to_owned(),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        thread.snapshot.pagination.forward,
        TimelinePageState::Unavailable
    );
    assert!(!thread.snapshot.capabilities.paginate_forward);
    let paged = owner
        .paginate(NativeTimelineViewPaginationRequest {
            stream_id: thread.stream_id.clone(),
            direction: NativeTimelineDirection::Forwards,
        })
        .await
        .expect("a thread's forward page is a no-op, not a failure");
    assert_eq!(paged.pagination.forward, TimelinePageState::Unavailable);

    // Event focus: the SDK's own end-of-timeline answer.
    let f = EventFactory::new().room(room_id);
    let ancient = event_id!("$forward-ancient");
    let ancient_event = || {
        f.text_msg("ancient message")
            .sender(*BOB)
            .event_id(ancient)
            .into_event()
    };
    server
        .mock_room_event()
        .room(room_id)
        .match_event_id()
        .ok(ancient_event())
        .mount()
        .await;
    server
        .mock_room_event_context()
        .room(room_id)
        .match_event_id()
        .ok(RoomContextResponseTemplate::new(ancient_event())
            .start("forward-prev")
            .end("forward-next"))
        .mount()
        .await;
    let focused = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: ancient.to_string(),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        focused.position,
        TimelineViewPosition::Focused {
            target_event_id: ancient.to_string()
        },
        "an event-focused window, or the proof is vacuous"
    );
    assert_eq!(
        focused.snapshot.pagination.forward,
        TimelinePageState::Available
    );
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default()
            .events(vec![f
                .text_msg("one page newer")
                .sender(*BOB)
                .event_id(event_id!("$forward-newer"))
                .into_raw_timeline()])
            .end_token("forward-later"))
        .mock_once()
        .mount()
        .await;
    let paged = owner
        .paginate(NativeTimelineViewPaginationRequest {
            stream_id: focused.stream_id.clone(),
            direction: NativeTimelineDirection::Forwards,
        })
        .await
        .unwrap();
    assert_eq!(paged.pagination.forward, TimelinePageState::Available);
    assert!(message_event_ids(&paged.rows).contains(&"$forward-newer".to_owned()));
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mock_once()
        .mount()
        .await;
    let paged = owner
        .paginate(NativeTimelineViewPaginationRequest {
            stream_id: focused.stream_id.clone(),
            direction: NativeTimelineDirection::Forwards,
        })
        .await
        .unwrap();
    assert_eq!(paged.pagination.forward, TimelinePageState::Exhausted);
}

#[tokio::test]
async fn jump_latest_keeps_the_old_stream_until_the_new_open_succeeds() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!jump-latest:example.org");
    // One cached event: a fresh live build probes one history page.
    cached_room(&server, &client, room_id, 1).await;
    let (owner, _batches) = owner_with_batches(&client, 75);
    let opened = owner.open_at(live_bottom(room_id)).await.unwrap();

    // Force the next open to rebuild the live timeline, and make its
    // history probe slow so the jump is still opening when cancelled.
    owner.registry.lock().await.entries.remove(room_id.as_str());
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().with_delay(Duration::from_secs(3)))
        .mount()
        .await;
    let jump = timeout(
        Duration::from_millis(300),
        owner.jump_latest(NativeTimelineJumpLatestRequest {
            stream_id: opened.stream_id.clone(),
        }),
    )
    .await;
    assert!(jump.is_err(), "the jump was still opening");
    owner
        .snapshot(&opened.stream_id)
        .await
        .expect("the old stream survives an open that did not finish");

    let jumped = timeout(
        Duration::from_secs(10),
        owner.jump_latest(NativeTimelineJumpLatestRequest {
            stream_id: opened.stream_id.clone(),
        }),
    )
    .await
    .expect("the jump finishes")
    .unwrap();
    assert_ne!(jumped.stream_id, opened.stream_id);
    assert_eq!(
        owner.snapshot(&opened.stream_id).await.unwrap_err(),
        "v-timeline-view-not-open",
        "a successful jump closes the stream it replaced"
    );
    owner.snapshot(&jumped.stream_id).await.unwrap();
}

#[tokio::test]
async fn follow_live_moves_the_update_task_to_the_live_position() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!follow-position:example.org");
    let older = event_id!("$follow-older");
    let newer = event_id!("$follow-newer");
    let f = EventFactory::new().room(room_id);
    let fully_read = |event_id: &str| {
        Raw::<AnyRoomAccountDataEvent>::from_json_string(
            serde_json::json!({"type": "m.fully_read", "content": {"event_id": event_id}})
                .to_string(),
        )
        .unwrap()
    };
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .set_unread_notifications_count(serde_json::json!({"notification_count": 1}))
                .add_account_data(fully_read(older.as_str()))
                .add_timeline_event(f.text_msg("older").sender(*BOB).event_id(older))
                .add_timeline_event(f.text_msg("newer").sender(*BOB).event_id(newer)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    let (owner, mut batches) = owner_with_batches(&client, 76);
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::Unread,
        })
        .await
        .unwrap();
    assert_eq!(
        opened.position,
        TimelineViewPosition::Unread {
            anchor_event_id: older.to_string()
        }
    );
    let followed = owner
        .follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: newer.to_string(),
        })
        .await
        .unwrap();
    assert_eq!(followed.position, TimelineViewPosition::LiveBottom);

    // A marker outside the window has no visible anchor, so the stream's
    // position decides the anchor: the live position reports the marker
    // itself, never the open-time unread anchor.
    let arrival = event_id!("$follow-arrival");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_account_data(fully_read("$follow-unknown"))
                .add_timeline_event(f.text_msg("arrival").sender(*BOB).event_id(arrival)),
        )
        .await;
    let batch = timeout(WAIT, async {
        loop {
            let batch = batches.recv().await.expect("view update stream ended");
            if batch.ops.iter().any(|op| match op {
                TimelineViewDeltaOp::PushBack { row }
                | TimelineViewDeltaOp::Insert { row, .. }
                | TimelineViewDeltaOp::Set { row, .. } => {
                    message_event_ids(std::slice::from_ref(row)) == [arrival.to_string()]
                }
                TimelineViewDeltaOp::Append { rows } => {
                    message_event_ids(rows).contains(&arrival.to_string())
                }
                _ => false,
            }) {
                break batch;
            }
        }
    })
    .await
    .expect("the arrival batch");
    assert_eq!(
        batch.read_state.unwrap().unread_anchor_event_id.as_deref(),
        Some("$follow-unknown")
    );
}

#[tokio::test]
async fn streams_whose_batches_go_unread_expire_and_are_reaped() {
    const TTL: Duration = Duration::from_millis(250);
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!unclaimed:example.org");
    cached_room(&server, &client, room_id, 2).await;
    let (sender, _batches) = unbounded_channel();
    let owner = NativeTimelineOwner::new(
        &client,
        Arc::new(move |batch| {
            let _ = sender.send(batch);
        }),
        77,
    )
    .with_unclaimed_stream_ttl(TTL);
    let read = owner.open_at(live_bottom(room_id)).await.unwrap();
    let unread = owner.open_at(live_bottom(room_id)).await.unwrap();

    for round in 0..3 {
        sync_messages(&server, &client, room_id, [format!("round {round}")]).await;
        // The consumer of `read` reads every batch; nobody reads `unread`,
        // whose first unread batch is older than the TTL by the next round.
        until_snapshot(&owner, &read.stream_id, |snapshot| {
            message_event_ids(&snapshot.rows).len() == 3 + round
        })
        .await;
        tokio::time::sleep(TTL + Duration::from_millis(50)).await;
    }
    timeout(WAIT, async {
        while !owner.registry.lock().await.view_streams[&unread.stream_id]
            .view
            .is_expired()
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the unread stream expires");
    assert_eq!(
        owner.snapshot(&unread.stream_id).await.unwrap_err(),
        "v-timeline-view-not-open"
    );
    owner.snapshot(&read.stream_id).await.unwrap();

    let next = owner.open_at(live_bottom(room_id)).await.unwrap();
    let registry = owner.registry.lock().await;
    assert!(!registry.view_streams.contains_key(&unread.stream_id));
    assert!(registry.view_streams.contains_key(&read.stream_id));
    assert!(registry.view_streams.contains_key(&next.stream_id));
}

#[tokio::test]
async fn a_full_utd_index_still_opens_the_room() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!utd-cap:example.org");
    let f = EventFactory::new().room(room_id);
    let encrypted = f
        .event(RoomEncryptedEventContent::new(
            EncryptedEventScheme::MegolmV1AesSha2(
                MegolmV1AesSha2ContentInit {
                    ciphertext: "AwgAEtABPRMavuZMDJrPo6pGQP4qVmpcuapuXtzKXJyi3YpEsjSWdzuRKIgJzD4P"
                        .to_owned(),
                    sender_key: "DeHIg4gwhClxzFYcmNntPNF9YtsdZbmMy8+3kzCMXHA".to_owned(),
                    device_id: "NLAZCWIOCO".into(),
                    session_id: "gM8i47Xhu0q52xLfgUXzanCMpLinoyVyH7R58cBuVBU".into(),
                }
                .into(),
            ),
            None,
        ))
        .sender(*BOB)
        .event_id(event_id!("$utd-cap-target"));
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("clear").sender(*BOB))
                .add_timeline_event(encrypted),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;

    let mut registry = NativeTimelineRegistry::new(78);
    for index in 0..MAX_UTD_ENTRIES {
        registry
            .utd_index
            .mark_unavailable(
                TimelineEncryptedUnavailableItem {
                    item_id: format!("filler-item-{index}"),
                    event_id: format!("$filler-{index}"),
                    room_id: "!filler:example.org".to_owned(),
                    reason: None,
                },
                UtdReasonCode::MissingKeys,
            )
            .unwrap();
    }

    // The legacy snapshot indexes UTD rows; a full index must not fail it.
    let legacy = registry
        .open(&client, room_id.as_str())
        .await
        .expect("a full UTD index never fails a room open");
    assert!(
        legacy
            .items
            .iter()
            .any(|item| item.decryption_state.is_some()),
        "the room shows a UTD row, or the proof is vacuous"
    );
    assert_eq!(legacy.utd.pending_count + legacy.utd.unavailable_count, 1);
    assert!(registry
        .utd_index
        .get(room_id.as_str(), "$utd-cap-target")
        .is_none());

    // View opens never touch the legacy index.
    registry
        .open_at(Arc::new(|_| {}), &client, live_bottom(room_id), None)
        .await
        .expect("a view open succeeds with a full UTD index");
}
