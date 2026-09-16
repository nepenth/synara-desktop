//! Recover remote `m.reaction` annotation ids for viewer/moderator redact.
//!
//! 0.19 `ReactionInfo` omits remote ids (`send_state: None`). This test fills
//! them from the room event cache and `/relations`, then redacts by the
//! recovered id. Self-unreact stays `toggle_reaction` and does not wait on
//! a missing id (the live Synapse proof uses that path).

#![recursion_limit = "256"]

use std::{sync::Arc, time::Duration};

use matrix_sdk::room::IncludeRelations;
use matrix_sdk::test_utils::mocks::{
    MatrixMockServer, RoomContextResponseTemplate, RoomMessagesResponseTemplate,
    RoomRelationsResponseTemplate,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::{
    event_id,
    events::{relation::RelationType, TimelineEventType},
    room_id, RoomVersionId,
};
use synara_core::app::timeline::{
    NativeReactionMutation, NativeTimelineOpenPosition, NativeTimelineOpenRequest,
    NativeTimelineOwner, TimelineReaction, TimelineViewRow,
};

fn message_reactions<'a>(rows: &'a [TimelineViewRow], event_id: &str) -> &'a [TimelineReaction] {
    for row in rows {
        if let TimelineViewRow::Message(row) = row {
            if row.event.event_id.as_deref() == Some(event_id) {
                return &row.reactions;
            }
        }
    }
    panic!("missing projected message {event_id}");
}

fn sender_reaction_id(reactions: &[TimelineReaction], key: &str, user_id: &str) -> Option<String> {
    reactions
        .iter()
        .find(|reaction| reaction.key == key)
        .and_then(|reaction| {
            reaction
                .senders
                .iter()
                .find(|sender| sender.user_id == user_id)
                .and_then(|sender| sender.reaction_event_id.clone())
        })
}

#[tokio::test]
async fn remote_reaction_ids_recover_for_other_sender_and_toggle_unreact_stays_id_free() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!reactions-core:example.org");
    let target = event_id!("$reaction-target");
    let bob_reaction = event_id!("$bob-thumbsup");
    let own_reaction = event_id!("$own-thumbsup");
    let f = EventFactory::new().room(room_id);
    let own_user_id = client.user_id().unwrap().to_owned();

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .add_timeline_event(
                    f.text_msg("earlier").sender(*BOB).event_id(event_id!("$earlier")),
                )
                .add_timeline_event(f.text_msg("target").sender(*BOB).event_id(target))
                .add_timeline_event(
                    f.reaction(target, "👍")
                        .sender(*BOB)
                        .event_id(bob_reaction),
                ),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    server.mock_room_send().ok(own_reaction).mount().await;
    server
        .mock_room_redact()
        .ok(event_id!("$reaction-redaction"))
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
        .events_before(vec![])
        .events_after(vec![])
        .start("reactions-prev")
        .end("reactions-next"))
        .mount()
        .await;
    server
        .mock_room_relations()
        .match_target_event(target.to_owned())
        .match_subrequest(IncludeRelations::RelationsOfTypeAndEventType(
            RelationType::Annotation,
            TimelineEventType::Reaction,
        ))
        .ok(RoomRelationsResponseTemplate::default().events(vec![f
            .reaction(target, "👍")
            .sender(*BOB)
            .event_id(bob_reaction)
            .into_raw_timeline()]))
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 31);
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .expect("open native timeline");

    let bob_id = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let snapshot = owner
                .snapshot(&opened.stream_id)
                .await
                .unwrap_or_else(|_| opened.snapshot.clone());
            if let Some(id) = sender_reaction_id(
                message_reactions(&snapshot.rows, target.as_str()),
                "👍",
                BOB.as_str(),
            ) {
                return id;
            }
            let readback = owner
                .event_readback(room_id.as_str(), target.as_str())
                .await
                .ok();
            if let Some(id) = readback.as_ref().and_then(|readback| {
                readback
                    .item
                    .reactions
                    .iter()
                    .find(|reaction| reaction.key == "👍")
                    .and_then(|reaction| {
                        reaction
                            .senders
                            .iter()
                            .find(|sender| sender.user_id == BOB.as_str())
                            .and_then(|sender| sender.reaction_event_id.clone())
                    })
            }) {
                return id;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("recovered Bob's remote annotation id");
    assert_eq!(bob_id, bob_reaction.as_str());

    let redacted = owner
        .redact_reaction(
            room_id.as_str(),
            target.as_str(),
            bob_id.as_str(),
            "👍",
        )
        .await
        .expect("redact recovered annotation");
    assert_eq!(redacted.mutation, NativeReactionMutation::Redacted);

    let added = owner
        .toggle_reaction(room_id.as_str(), target.as_str(), "🎉")
        .await
        .expect("toggle add");
    assert_eq!(added.mutation, NativeReactionMutation::Added);
    let removed = owner
        .toggle_reaction(room_id.as_str(), target.as_str(), "🎉")
        .await
        .expect("toggle remove without waiting for a remote id");
    assert_eq!(removed.mutation, NativeReactionMutation::Removed);
}
