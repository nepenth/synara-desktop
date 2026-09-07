//! Deterministic proof for the read-state follow-live transition.
//!
//! A focused provider must never become live merely because its loaded tail
//! was painted. Promotion is valid only for unread placement on the owner's
//! actual live SDK provider, with an exact observation of its current tail.

use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomContextResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::{event_id, events::AnyRoomAccountDataEvent, room_id, serde::Raw, RoomVersionId};
use synara_core::app::timeline::{
    NativeTimelineFollowLiveRequest, NativeTimelineOpenPosition, NativeTimelineOpenRequest,
    NativeTimelineOwner, TimelineViewPosition,
};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

async fn focused_owner() -> (
    MatrixMockServer,
    matrix_sdk::Client,
    NativeTimelineOwner,
    String,
    String,
    String,
) {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!follow-live:example.org");
    let older_id = event_id!("$follow-live-older");
    let newer_id = event_id!("$follow-live-newer");
    let f = EventFactory::new().room(room_id);
    let own_user_id = client.user_id().unwrap().to_owned();
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .set_unread_notifications_count(serde_json::json!({"notification_count": 1}))
                .add_account_data(
                    Raw::<AnyRoomAccountDataEvent>::from_json_string(
                        serde_json::json!({
                            "type": "m.fully_read", "content": {"event_id": older_id}
                        })
                        .to_string(),
                    )
                    .unwrap(),
                )
                .add_timeline_event(f.text_msg("older message").sender(*BOB).event_id(older_id))
                .add_timeline_event(f.text_msg("newer message").sender(*BOB).event_id(newer_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    // Focused opens resolve their anchor through /context, not sync: serve
    // the anchor with the newer message after it. Even when this focused
    // window reaches the current sync tail, its provider is still not live.
    server
        .mock_room_event_context()
        .room(room_id)
        .ok(RoomContextResponseTemplate::new(
            f.text_msg("older message")
                .sender(*BOB)
                .event_id(older_id)
                .into_event(),
        )
        .events_before(vec![])
        .events_after(vec![f
            .text_msg("newer message")
            .sender(*BOB)
            .event_id(newer_id)
            .into_event()])
        .start("follow-live-prev")
        .end("follow-live-next"))
        .mount()
        .await;

    let (_updates, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let owner = NativeTimelineOwner::new(
        &client,
        Arc::new(move |batch| {
            let _ = _updates.send(batch);
        }),
        7,
    );
    (
        server,
        client,
        owner,
        room_id.to_string(),
        older_id.to_string(),
        newer_id.to_string(),
    )
}

#[tokio::test]
async fn follow_live_rejects_a_focused_provider_even_on_the_exact_room_tail() {
    let (_server, _client, owner, room_id, _older_id, newer_id) = focused_owner().await;
    owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.clone(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .expect("live provider must be registered");
    let older = event_id!("$follow-live-older");
    let opened = timeout(
        WAIT,
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.clone(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: older.to_string(),
            },
        }),
    )
    .await
    .expect("open must complete")
    .expect("focused open must succeed");
    assert_ne!(
        opened.position,
        TimelineViewPosition::LiveBottom,
        "a focused open must not already be live or the flip proof is vacuous"
    );

    let error = timeout(
        WAIT,
        owner.follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: newer_id.clone(),
        }),
    )
    .await
    .expect("follow must complete")
    .expect_err("a focused SDK provider must remain focused");
    assert_eq!(error, "v-timeline-follow-live-tail-not-loaded");
}

#[tokio::test]
async fn follow_live_fails_closed_on_a_stale_observed_tail() {
    let (_server, _client, owner, room_id, older_id, _newer_id) = focused_owner().await;
    let older = event_id!("$follow-live-older");
    let opened = timeout(
        WAIT,
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.clone(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: older.to_string(),
            },
        }),
    )
    .await
    .expect("open must complete")
    .expect("focused open must succeed");

    let error = timeout(
        WAIT,
        owner.follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: older_id.clone(),
        }),
    )
    .await
    .expect("follow must complete")
    .expect_err("a stale tail must fail closed");
    assert_eq!(error, "v-timeline-follow-live-tail-not-loaded");

    let missing = timeout(
        WAIT,
        owner.follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: "no-such-stream".to_owned(),
            observed_live_tail_event_id: older_id.clone(),
        }),
    )
    .await
    .expect("follow must complete")
    .expect_err("an unknown stream must fail closed");
    assert_eq!(missing, "v-timeline-view-not-open");

    let blank = timeout(
        WAIT,
        owner.follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: "   ".to_owned(),
        }),
    )
    .await
    .expect("follow must complete")
    .expect_err("a blank tail must fail closed");
    assert_eq!(blank, "v-timeline-follow-live-tail-required");
}

#[tokio::test]
async fn follow_live_is_idempotent_on_an_already_live_stream() {
    let (_server, _client, owner, room_id, _older_id, _newer_id) = focused_owner().await;
    let opened = timeout(
        WAIT,
        owner.open_at(NativeTimelineOpenRequest {
            room_id: room_id.clone(),
            position: NativeTimelineOpenPosition::LiveBottom,
        }),
    )
    .await
    .expect("open must complete")
    .expect("live open must succeed");
    assert_eq!(opened.position, TimelineViewPosition::LiveBottom);

    // A stale observation against an already-live stream still succeeds: the
    // receipt path performs its own exact-tail check, so follow-live must not
    // invent a failure for a transition that has nothing to do.
    let snapshot = timeout(
        WAIT,
        owner.follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: "$stale-not-the-tail".to_owned(),
        }),
    )
    .await
    .expect("follow must complete")
    .expect("live streams follow idempotently");
    assert_eq!(snapshot.position, TimelineViewPosition::LiveBottom);
}

#[tokio::test]
async fn follow_live_promotes_unread_placement_and_keeps_receiving_sync() {
    let (server, client, owner, room_id, older_id, newer_id) = focused_owner().await;
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.clone(),
            position: NativeTimelineOpenPosition::Unread,
        })
        .await
        .expect("unread open must succeed");
    assert!(matches!(
        opened.position,
        TimelineViewPosition::Unread { .. }
    ));
    let error = owner
        .follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: older_id,
        })
        .await
        .expect_err("a stale live observation must fail");
    assert_eq!(error, "v-timeline-follow-live-tail-not-loaded");
    assert!(matches!(
        owner.snapshot(&opened.stream_id).await.unwrap().position,
        TimelineViewPosition::Unread { .. }
    ));

    let followed = owner
        .follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: newer_id,
        })
        .await
        .expect("exact live tail promotes unread placement");
    assert_eq!(followed.position, TimelineViewPosition::LiveBottom);

    let room_id = room_id!("!follow-live:example.org");
    let next_id = event_id!("$follow-live-arrival");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(
                f.text_msg("arrives after follow")
                    .sender(*BOB)
                    .event_id(next_id),
            ),
        )
        .await;
    timeout(WAIT, async {
        loop {
            let snapshot = owner.snapshot(&opened.stream_id).await.unwrap();
            if snapshot.rows.iter().any(|row| {
                matches!(row,
                synara_core::app::timeline::TimelineViewRow::Message(message)
                if message.event.event_id.as_deref() == Some(next_id.as_str()))
            }) {
                assert_eq!(snapshot.position, TimelineViewPosition::LiveBottom);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("promoted provider must still receive live events");
}

#[tokio::test]
async fn visible_frontier_reads_folded_edits_and_reactions_but_not_unseen_new_messages() {
    use ruma::events::room::message::RoomMessageEventContent;
    use synara_core::app::timeline::{
        NativeTimelineReadAction, NativeTimelineReadIntent, NativeTimelineReadStateRequest,
    };

    let (server, client, owner, room, _, message) = focused_owner().await;
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room.clone(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .unwrap();
    server.mock_send_read_markers().ok().expect(3).mount().await;
    let mark = |event: String| NativeTimelineReadStateRequest {
        stream_id: opened.stream_id.clone(),
        action: NativeTimelineReadAction::MarkRead,
        intent: NativeTimelineReadIntent::AutomaticVisibility,
        observed_live_tail_event_id: Some(event),
    };
    let initial = owner.set_read_state(mark(message.clone())).await.unwrap();
    assert_eq!(
        initial.acknowledged_event_id.as_deref(),
        Some(message.as_str())
    );

    let room_id = room_id!("!follow-live:example.org");
    let message_id = event_id!("$follow-live-newer");
    let edit_id = event_id!("$folded-edit");
    let reaction_id = event_id!("$folded-reaction");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(
                f.text_msg("* tool update")
                    .sender(*BOB)
                    .event_id(edit_id)
                    .edit(
                        message_id,
                        RoomMessageEventContent::text_plain("tool update").into(),
                    ),
            ),
        )
        .await;
    for expected in [edit_id, reaction_id] {
        if expected == reaction_id {
            server
                .sync_room(
                    &client,
                    JoinedRoomBuilder::new(room_id).add_timeline_event(
                        f.reaction(message_id, "👍")
                            .sender(*BOB)
                            .event_id(reaction_id),
                    ),
                )
                .await;
        }
        let snapshot = timeout(WAIT, async {
            loop {
                let snapshot = owner.snapshot(&opened.stream_id).await.unwrap();
                if snapshot.read_state.receipt_tail_event_id.as_deref() == Some(expected.as_str()) {
                    break snapshot;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("folded event must reach the SDK read frontier");
        assert_eq!(
            snapshot.read_state.visible_tail_event_id.as_deref(),
            Some(message.as_str())
        );
        assert!(
            !snapshot.rows.iter().any(|row| matches!(row,
            synara_core::app::timeline::TimelineViewRow::Message(item)
                if item.event.event_id.as_deref() == Some(expected.as_str()))),
            "the receipt target must not masquerade as a separately displayed message"
        );
        let readback = owner
            .set_read_state(mark(snapshot.read_state.receipt_tail_event_id.unwrap()))
            .await
            .unwrap();
        assert_eq!(readback.receipt_sent, Some(true));
        assert_eq!(
            readback.acknowledged_event_id.as_deref(),
            Some(expected.as_str())
        );
    }

    // Capture the old observation, then deliver a real new message before its
    // automatic write executes. Even though Core now knows that message, the
    // old visibility token cannot acknowledge it.
    let unseen = event_id!("$unseen-after-frontier");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(f.text_msg("not yet painted").sender(*BOB).event_id(unseen)),
        )
        .await;
    timeout(WAIT, async {
        loop {
            let snapshot = owner.snapshot(&opened.stream_id).await.unwrap();
            if snapshot.read_state.receipt_tail_event_id.as_deref() == Some(unseen.as_str()) {
                assert_eq!(
                    snapshot.read_state.visible_tail_event_id.as_deref(),
                    Some(unseen.as_str())
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let rejected = owner
        .set_read_state(mark(reaction_id.to_string()))
        .await
        .unwrap();
    assert_eq!(rejected.receipt_sent, Some(false));
    assert_eq!(rejected.acknowledged_event_id, None);

    let requests = server.server().received_requests().await.unwrap();
    let markers: Vec<serde_json::Value> = requests
        .iter()
        .filter(|request| request.url.path().ends_with("/read_markers"))
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect();
    assert_eq!(markers.len(), 3);
    for (body, expected) in markers.iter().zip([message_id, edit_id, reaction_id]) {
        assert_eq!(body["m.fully_read"], expected.as_str());
        assert_eq!(body["m.read.private"], expected.as_str());
        assert!(
            body.get("m.read").is_none(),
            "automatic acknowledgement must remain private"
        );
    }
}

#[tokio::test]
async fn last_read_hidden_edit_resolves_at_its_stream_position_not_the_edited_old_row() {
    use ruma::events::room::message::RoomMessageEventContent;
    let (server, client, owner, room, older, newer) = focused_owner().await;
    let room_id = room_id!("!follow-live:example.org");
    let old_id = ruma::EventId::parse(&older).unwrap();
    let edit_id = event_id!("$edit-of-old-row");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(
                    f.text_msg("* edited old message")
                        .sender(*BOB)
                        .event_id(edit_id)
                        .edit(
                            &old_id,
                            RoomMessageEventContent::text_plain("edited old message").into(),
                        ),
                )
                .add_account_data(
                    Raw::<AnyRoomAccountDataEvent>::from_json_string(
                        serde_json::json!({
                            "type": "m.fully_read", "content": { "event_id": edit_id }
                        })
                        .to_string(),
                    )
                    .unwrap(),
                ),
        )
        .await;
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room,
            position: NativeTimelineOpenPosition::Normal {
                viewport: Default::default(),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        opened.snapshot.read_state.own_read_event_id.as_deref(),
        Some(edit_id.as_str())
    );
    assert_eq!(
        opened.snapshot.read_state.unread_anchor_event_id.as_deref(),
        Some(newer.as_str())
    );
    assert_ne!(
        opened.snapshot.read_state.unread_anchor_event_id.as_deref(),
        Some(older.as_str())
    );
}

#[tokio::test]
async fn newer_comparable_receipt_wins_over_fully_read_for_restoration() {
    use ruma::events::receipt::{ReceiptThread, ReceiptType};
    let (server, client, owner, room, _older, newer) = focused_owner().await;
    let room_id = room_id!("!follow-live:example.org");
    let newer_id = ruma::EventId::parse(&newer).unwrap();
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_receipt(
                f.read_receipts()
                    .add(
                        &newer_id,
                        client.user_id().unwrap(),
                        ReceiptType::ReadPrivate,
                        ReceiptThread::Unthreaded,
                    )
                    .into_event(),
            ),
        )
        .await;
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room,
            position: NativeTimelineOpenPosition::Normal {
                viewport: Default::default(),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        opened.snapshot.read_state.own_read_event_id.as_deref(),
        Some(newer.as_str())
    );
    assert_eq!(
        opened.snapshot.read_state.unread_anchor_event_id.as_deref(),
        Some(newer.as_str())
    );
}

#[tokio::test]
async fn missing_last_read_retains_live_rows_and_an_explicit_navigation_target() {
    let (server, client, owner, room, _older, newer) = focused_owner().await;
    let room_id = room_id!("!follow-live:example.org");
    let missing = event_id!("$unavailable-read-location");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_account_data(
                Raw::<AnyRoomAccountDataEvent>::from_json_string(
                    serde_json::json!({
                        "type": "m.fully_read", "content": { "event_id": missing }
                    })
                    .to_string(),
                )
                .unwrap(),
            ),
        )
        .await;
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room,
            position: NativeTimelineOpenPosition::Normal {
                viewport: Default::default(),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        opened.position,
        TimelineViewPosition::Unread {
            anchor_event_id: missing.to_string()
        }
    );
    assert_eq!(
        opened.snapshot.read_state.visible_tail_event_id.as_deref(),
        Some(newer.as_str())
    );
    assert_eq!(
        opened.snapshot.read_state.unread_anchor_event_id.as_deref(),
        Some(missing.as_str())
    );
    assert!(
        !server
            .server()
            .received_requests()
            .await
            .unwrap()
            .iter()
            .any(|request| request.url.path().contains("/context/")),
        "normal missing-anchor open must not replace live rows with a focused history request"
    );
}

#[tokio::test]
async fn one_cached_event_bootstraps_one_bounded_older_page_through_the_sdk() {
    use matrix_sdk::test_utils::mocks::RoomMessagesResponseTemplate;
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!sparse-history:example.org");
    let f = EventFactory::new().room(room_id);
    let newest = event_id!("$sparse-newest");
    let older = event_id!("$sparse-older");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .set_timeline_prev_batch("older-page")
                .add_timeline_event(f.text_msg("cached message").sender(*BOB).event_id(newest)),
        )
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().events(vec![f
            .text_msg("older context")
            .sender(*BOB)
            .event_id(older)
            .into_raw_timeline()]))
        .expect(1)
        .mount()
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 1);
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .unwrap();
    assert!(opened.snapshot.rows.iter().any(|row| matches!(row,
        synara_core::app::timeline::TimelineViewRow::Message(item)
            if item.event.event_id.as_deref() == Some(older.as_str()))));
    assert_eq!(
        opened.snapshot.read_state.visible_tail_event_id.as_deref(),
        Some(newest.as_str())
    );
}
