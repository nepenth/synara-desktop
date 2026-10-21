//! Edits go through the SDK edit path (review S16): a queued message is
//! edited in place through `Timeline::edit`, and a sent one gets the
//! `make_edit_event` replacement `Timeline::edit` would send.

use super::*;
use matrix_sdk::ruma::{room_id, RoomVersionId};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::event_id;

async fn sent_bodies(server: &MatrixMockServer) -> Vec<serde_json::Value> {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|request| {
            request.method.as_str() == "PUT" && request.url.path().contains("/send/m.room.message/")
        })
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

#[tokio::test]
async fn editing_a_queued_message_replaces_it_before_it_is_sent() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s16-local:example.org");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let room = client.get_room(room_id).unwrap();
    // Hold the message in the queue, as a slow or offline send would.
    room.send_queue().set_enabled(false);
    let txn = enqueue_event_via_room_queue(
        &room,
        RoomMessageEventContent::text_plain("teh typo").into(),
    )
    .await
    .unwrap()
    .transaction_id;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 41);
    let result = timeout(
        Duration::from_secs(8),
        owner.edit_message(
            room_id.to_string(),
            String::new(),
            "the typo, fixed".to_owned(),
            None,
            None,
            None,
            None,
            None,
            Some(txn.clone()),
        ),
    )
    .await
    .expect("edit should finish")
    .expect("a queued message can be edited");
    assert_eq!(result.status, crate::dto::SendStatus::Queued);
    assert_eq!(result.local_txn_id, txn);

    server
        .mock_room_send()
        .ok(event_id!("$s16-sent"))
        .mock_once()
        .mount()
        .await;
    room.send_queue().set_enabled(true);
    timeout(Duration::from_secs(8), async {
        while sent_bodies(&server).await.is_empty() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the queued message is sent");
    let bodies = sent_bodies(&server).await;
    assert_eq!(bodies.len(), 1, "one message, not a message plus an edit");
    assert_eq!(bodies[0]["body"], "the typo, fixed");
    assert!(bodies[0].get("m.new_content").is_none());
}

#[tokio::test]
async fn editing_an_unknown_queued_message_without_an_event_id_fails() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s16-missing:example.org");
    server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 42);
    let error = timeout(
        Duration::from_secs(8),
        owner.edit_message(
            room_id.to_string(),
            String::new(),
            "edit".to_owned(),
            None,
            None,
            None,
            None,
            None,
            Some("unknown-txn".to_owned()),
        ),
    )
    .await
    .expect("edit should finish")
    .unwrap_err();
    assert_eq!(error, "v-send.r-edit-target-not-found");
}

#[tokio::test]
async fn editing_a_sent_message_uses_the_sdk_replacement_and_notifies_new_mentions_once() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s16-remote:example.org");
    let original_id = event_id!("$s16-original");
    let own = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own, RoomVersionId::V11))
                .add_timeline_event(
                    f.text_msg("hi bob")
                        .sender(&own)
                        .event_id(original_id)
                        .mentions(Mentions::with_user_ids([BOB.to_owned()])),
                ),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_send()
        .ok(event_id!("$s16-edit"))
        .mock_once()
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 43);
    let result = timeout(
        Duration::from_secs(8),
        owner.edit_message(
            room_id.to_string(),
            original_id.to_string(),
            "hi bob and carol".to_owned(),
            Some("m.notice".to_owned()),
            None,
            Some(vec![BOB.to_string(), "@carol:example.org".to_owned()]),
            None,
            None,
            None,
        ),
    )
    .await
    .expect("edit should finish")
    .expect("own message edit");
    assert_eq!(result.status, crate::dto::SendStatus::Sent);
    assert_eq!(result.event_id, "$s16-edit");

    let bodies = sent_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    let edit = &bodies[0];
    assert_eq!(edit["m.relates_to"]["rel_type"], "m.replace");
    assert_eq!(edit["m.relates_to"]["event_id"], original_id.as_str());
    assert_eq!(edit["m.new_content"]["msgtype"], "m.notice");
    assert_eq!(edit["m.new_content"]["body"], "hi bob and carol");
    assert_eq!(
        edit["m.new_content"]["m.mentions"]["user_ids"],
        serde_json::json!([BOB.as_str(), "@carol:example.org"])
    );
    // Bob was already mentioned; only Carol is notified by the edit.
    assert_eq!(
        edit["m.mentions"]["user_ids"],
        serde_json::json!(["@carol:example.org"])
    );
}

#[tokio::test]
async fn editing_someone_elses_message_is_refused() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!s16-not-own:example.org");
    let theirs = event_id!("$s16-theirs");
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("bob's").sender(*BOB).event_id(theirs)),
        )
        .await;
    server.mock_room_send().ok(theirs).expect(0).mount().await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 44);
    let error = owner
        .edit_message(
            room_id.to_string(),
            theirs.to_string(),
            "mine now".to_owned(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error, "v-send.r-edit-not-own-event");
}
