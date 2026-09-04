//! Deterministic proof for the read-state follow-live transition.
//!
//! A room opened at a non-live position (focused history anchor) can never
//! satisfy automatic read receipts: both the presenter gate and Core require a
//! live-bottom stream. These tests drive the pinned `matrix-sdk-ui` timeline
//! against a mocked homeserver and prove Core flips the stream position only
//! on an exact SDK-tail observation — never on a stale tail, and idempotently
//! when already live.

use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomContextResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::{event_id, room_id, RoomVersionId};
use synara_core::app::timeline::{
    NativeTimelineFollowLiveRequest, NativeTimelineOpenPosition, NativeTimelineOpenRequest,
    NativeTimelineOwner, TimelineViewPosition,
};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

async fn focused_owner() -> (
    MatrixMockServer,
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
                .add_timeline_event(f.text_msg("older message").sender(*BOB).event_id(older_id))
                .add_timeline_event(f.text_msg("newer message").sender(*BOB).event_id(newer_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    // Focused opens resolve their anchor through /context, not sync: serve
    // the anchor with the newer message after it so the focused controller's
    // live tail is the newer event.
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
        owner,
        room_id.to_string(),
        older_id.to_string(),
        newer_id.to_string(),
    )
}

#[tokio::test]
async fn follow_live_flips_a_focused_stream_on_the_exact_sdk_tail() {
    let (_server, owner, room_id, _older_id, newer_id) = focused_owner().await;
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

    let snapshot = timeout(
        WAIT,
        owner.follow_live_tail(NativeTimelineFollowLiveRequest {
            stream_id: opened.stream_id.clone(),
            observed_live_tail_event_id: newer_id.clone(),
        }),
    )
    .await
    .expect("follow must complete")
    .expect("exact-tail follow must succeed");
    assert_eq!(snapshot.position, TimelineViewPosition::LiveBottom);
    assert_eq!(snapshot.room_id.as_str(), room_id.as_str());
}

#[tokio::test]
async fn follow_live_fails_closed_on_a_stale_observed_tail() {
    let (_server, owner, room_id, older_id, _newer_id) = focused_owner().await;
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
    let (_server, owner, room_id, _older_id, _newer_id) = focused_owner().await;
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
