//! Timeline rows carry the SDK send state of this account's own sends.
//!
//! A local echo projects Sending, then Failed (recoverable) or Wedged with
//! a closed cause, under its SDK transaction id and without a server event
//! id. The remote echo keeps that transaction id from `unsigned`, so a shell
//! that shows its own pending row can always match it by transaction id,
//! even when the server echo replaces the local echo before the shell saw it.

use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::ruma::{event_id, room_id, TransactionId};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, ALICE};
use matrix_sdk_ui::timeline::{EventTimelineItem, RoomExt, Timeline};
use serde_json::json;
use synara_core::app::timeline::{project_event_row_base, NativeTimelineOwner};
use synara_core::dto::LocalEchoState;
use wiremock::ResponseTemplate;

/// The first timeline event matching `predicate`, waiting for it to appear.
async fn wait_for_event(
    timeline: &Timeline,
    predicate: impl Fn(&EventTimelineItem) -> bool,
) -> EventTimelineItem {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(event) = timeline
                .items()
                .await
                .iter()
                .filter_map(|item| item.as_event())
                .find(|event| predicate(event))
            {
                return event.clone();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("timeline event never appeared")
}

fn has_txn(event: &EventTimelineItem, txn: &str) -> bool {
    event.transaction_id().map(|id| id.as_str()) == Some(txn)
}

#[tokio::test]
async fn local_echo_rows_carry_send_state_cause_and_transaction_id() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!local-echo-rows:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    // Slow 413: the row is first Sending, then Wedged with its cause.
    server
        .mock_room_send()
        .respond_with(
            ResponseTemplate::new(413)
                .set_body_json(json!({
                    "errcode": "M_TOO_LARGE",
                    "error": "Request body too large",
                }))
                .set_delay(Duration::from_millis(500)),
        )
        .mock_once()
        .mount()
        .await;

    let timeline = room.timeline().await.expect("timeline");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 60);
    let sent = owner
        .send_text(
            room_id.to_string(),
            "too big".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("synara-client-row".into()),
        )
        .await
        .expect("queued");
    let txn = sent.local_txn_id.clone();

    let sending = wait_for_event(&timeline, |event| has_txn(event, &txn)).await;
    let row = project_event_row_base("sending-item", &sending);
    assert_eq!(row.local_echo_state, Some(LocalEchoState::Sending));
    assert_eq!(row.transaction_id.as_deref(), Some(txn.as_str()));
    assert_eq!(
        row.event_id, None,
        "a local echo has no server event id yet"
    );
    assert_eq!(row.send_failure, None);

    let wedged = wait_for_event(&timeline, |event| {
        has_txn(event, &txn)
            && project_event_row_base("probe", event).local_echo_state
                == Some(LocalEchoState::Wedged)
    })
    .await;
    let row = project_event_row_base("wedged-item", &wedged);
    assert_eq!(row.send_failure.as_deref(), Some("too_large"));
    assert_eq!(row.transaction_id.as_deref(), Some(txn.as_str()));
    assert_eq!(row.event_id, None);

    // Discard through the local-echo owner removes the row.
    assert!(owner
        .abort_send(room_id.as_str(), &txn)
        .await
        .expect("discard wedged echo"));
    tokio::time::timeout(Duration::from_secs(5), async {
        while timeline
            .items()
            .await
            .iter()
            .filter_map(|item| item.as_event())
            .any(|event| has_txn(event, &txn))
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("discarded echo must leave the timeline");
}

#[tokio::test]
async fn recoverable_local_echo_row_is_failed_with_its_cause() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!local-echo-recoverable:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "errcode": "M_UNKNOWN",
            "error": "unavailable",
        })))
        .mount()
        .await;

    let timeline = room.timeline().await.expect("timeline");
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 61);
    let txn = owner
        .send_text(
            room_id.to_string(),
            "later".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("queued")
        .local_txn_id;
    let failed = wait_for_event(&timeline, |event| {
        has_txn(event, &txn)
            && project_event_row_base("probe", event).local_echo_state
                == Some(LocalEchoState::Failed)
    })
    .await;
    let row = project_event_row_base("failed-item", &failed);
    assert_eq!(row.send_failure.as_deref(), Some("server_error"));
    assert_eq!(row.event_id, None);
}

#[tokio::test]
async fn own_remote_echo_row_keeps_the_unsigned_transaction_id() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!remote-echo-txn:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    let timeline = room.timeline().await.expect("timeline");
    let own_user = client.user_id().expect("user id").to_owned();
    let f = EventFactory::new().room(room_id);

    // The server echo arrives with no local echo before it (the case where
    // the remote echo replaced the local one before the shell appended it).
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(
                    f.text_msg("from this device")
                        .sender(&own_user)
                        .event_id(event_id!("$own-remote"))
                        .unsigned_transaction_id(<&TransactionId>::from("sdk-txn-remote")),
                )
                .add_timeline_event(
                    f.text_msg("from someone else")
                        .sender(*ALICE)
                        .event_id(event_id!("$other-remote"))
                        .unsigned_transaction_id(<&TransactionId>::from("not-ours")),
                ),
        )
        .await;

    let own = wait_for_event(&timeline, |event| {
        event.event_id() == Some(event_id!("$own-remote"))
    })
    .await;
    let row = project_event_row_base("own-item", &own);
    assert_eq!(row.transaction_id.as_deref(), Some("sdk-txn-remote"));
    assert_eq!(row.local_echo_state, None);
    assert_eq!(row.event_id.as_deref(), Some("$own-remote"));

    let other = wait_for_event(&timeline, |event| {
        event.event_id() == Some(event_id!("$other-remote"))
    })
    .await;
    assert_eq!(
        project_event_row_base("other-item", &other).transaction_id,
        None,
        "only this account's own events carry a transaction id"
    );
}
