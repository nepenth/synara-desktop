//! Deterministic proof that desktop notification decisions are owned by the
//! SDK-evaluated push rules inside Core.
//!
//! The renderer submits only `(room_id, event_id)` plus product strings. Core
//! loads the exact event, compares its sender with the bound session, and folds
//! the SDK push actions into notify / highlight / sound. Mode, mentions,
//! keywords, and mute semantics all come from the account's real `m.push_rules`
//! ruleset; no TypeScript reconstruction is consulted.

use matrix_sdk::test_utils::mocks::MatrixMockServer;
use matrix_sdk::Client;
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB, CAROL};
use ruma::push::{
    Action, EventMatchConditionData, NewConditionalPushRule, NewPushRule, NewSimplePushRule,
    PushCondition, Ruleset,
};
use ruma::{event_id, events::Mentions, room_id, OwnedRoomId, OwnedUserId, RoomVersionId};
use synara_core::app::notifications::{
    NativeNotificationDecideRequest, NativeNotificationDecisionOwner,
};

const ROOM_ID: &str = "!push-rules:example.org";

fn request(room_id: &str, event_id: Option<&str>) -> NativeNotificationDecideRequest {
    NativeNotificationDecideRequest {
        room_id: room_id.to_owned(),
        event_id: event_id.map(str::to_owned),
        kind: "message".to_owned(),
        title: "Product".to_owned(),
        body: "New inbox notification from Bob".to_owned(),
        route: Some(format!("/home/room/{room_id}")),
        suppress_if_focused_room: true,
    }
}

/// Serialize a ruleset as the global `m.push_rules` account-data event so the
/// SDK reads it from the same sync that carries the timeline events.
fn push_rules_account_data(rules: &Ruleset) -> serde_json::Value {
    serde_json::json!({
        "type": "m.push_rules",
        "content": { "global": rules },
    })
}

async fn synced_group_room() -> (MatrixMockServer, Client, EventFactory, OwnedUserId) {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!push-rules:example.org");
    let own_user_id = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(room_id);
    // Three joined members keep the room out of the one-to-one underride so
    // plain messages notify without a sound tweak; mentions add sound and
    // highlight. The own member event is required for the SDK push context.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .add_state_event(f.member(&own_user_id).display_name("Me"))
                .add_state_event(f.member(*BOB).display_name("Bob"))
                .add_state_event(f.member(*CAROL).display_name("Carol"))
                .add_timeline_event(
                    f.text_msg("hello everyone")
                        .sender(*BOB)
                        .event_id(event_id!("$plain")),
                )
                .add_timeline_event(
                    f.text_msg("hello me")
                        .sender(*BOB)
                        .mentions(Mentions::with_user_ids([own_user_id.clone()]))
                        .event_id(event_id!("$mention")),
                )
                .add_timeline_event(
                    f.text_msg("my own message")
                        .sender(&own_user_id)
                        .event_id(event_id!("$own")),
                ),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    (server, client, f, own_user_id)
}

#[tokio::test]
async fn default_rules_decide_notify_highlight_sound_and_own_events_in_core() {
    let (_server, client, _f, _own) = synced_group_room().await;
    let owner = NativeNotificationDecisionOwner::new(&client, 7).expect("owner binds session");

    let plain = owner
        .decide_observed(request(ROOM_ID, Some("$plain")))
        .await
        .expect("plain message decides");
    assert_eq!(plain.decision, "show");
    assert!(!plain.highlight, "a plain group message is not a highlight");
    assert!(!plain.sound, "the default group-message rule has no sound");
    let candidate = plain.candidate.expect("show carries a candidate");
    assert_eq!(candidate.room_id, ROOM_ID);
    assert_eq!(candidate.event_id.as_deref(), Some("$plain"));
    assert!(!candidate.is_encrypted);

    let mention = owner
        .decide_observed(request(ROOM_ID, Some("$mention")))
        .await
        .expect("mention decides");
    assert_eq!(mention.decision, "show");
    assert!(
        mention.highlight,
        "`.m.rule.is_user_mention` highlights through the SDK, not TypeScript"
    );
    assert!(mention.sound, "the SDK mention rule carries a sound tweak");

    let own = owner
        .decide_observed(request(ROOM_ID, Some("$own")))
        .await
        .expect("own message decides");
    assert_eq!(own.decision, "suppress");
    assert_eq!(own.reason.as_deref(), Some("own-event"));

    // Dedup is still owned by the wrapped index across a second observation.
    let again = owner
        .decide_observed(request(ROOM_ID, Some("$plain")))
        .await
        .expect("duplicate decides");
    assert_eq!(again.decision, "suppress");
    assert_eq!(again.reason.as_deref(), Some("duplicate-event"));
    assert_eq!(owner.pending_count().unwrap(), 2);
}

#[tokio::test]
async fn account_push_rules_own_mentions_only_and_mute_semantics() {
    let (server, client, f, own_user_id) = synced_group_room().await;
    let owner = NativeNotificationDecisionOwner::new(&client, 7).expect("owner binds session");
    let room_id: OwnedRoomId = room_id!("!push-rules:example.org").to_owned();

    // Mentions-only: a room rule with no actions silences ordinary messages
    // while the higher-priority default mention override still notifies.
    let mut mentions_only = Ruleset::server_default(&own_user_id);
    mentions_only
        .insert(
            NewPushRule::Room(NewSimplePushRule::new(room_id.clone(), vec![])),
            None,
            None,
        )
        .unwrap();
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_custom_global_account_data(push_rules_account_data(&mentions_only));
            builder.add_joined_room(
                JoinedRoomBuilder::new(&room_id)
                    .add_timeline_event(
                        f.text_msg("quiet update")
                            .sender(*BOB)
                            .event_id(event_id!("$plain-mentions-only")),
                    )
                    .add_timeline_event(
                        f.text_msg("hey me")
                            .sender(*BOB)
                            .mentions(Mentions::with_user_ids([own_user_id.clone()]))
                            .event_id(event_id!("$mention-mentions-only")),
                    ),
            );
        })
        .await;

    let quiet = owner
        .decide_observed(request(ROOM_ID, Some("$plain-mentions-only")))
        .await
        .expect("quiet message decides");
    assert_eq!(quiet.decision, "suppress");
    assert_eq!(
        quiet.reason.as_deref(),
        Some("push-rules-no-notify"),
        "the room rule silenced a plain message"
    );

    let mention = owner
        .decide_observed(request(ROOM_ID, Some("$mention-mentions-only")))
        .await
        .expect("mention decides");
    assert_eq!(mention.decision, "show");
    assert!(mention.highlight);

    // Mute: an override with no actions for this room precedes every mention
    // rule, so even a highlight-worthy event stays silent.
    let mut muted = Ruleset::server_default(&own_user_id);
    muted
        .insert(
            NewPushRule::Override(NewConditionalPushRule::new(
                room_id.to_string(),
                vec![PushCondition::EventMatch(EventMatchConditionData::new(
                    "room_id".to_owned(),
                    room_id.to_string(),
                ))],
                vec![],
            )),
            None,
            None,
        )
        .unwrap();
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_custom_global_account_data(push_rules_account_data(&muted));
            builder.add_joined_room(
                JoinedRoomBuilder::new(&room_id).add_timeline_event(
                    f.text_msg("hey me again")
                        .sender(*BOB)
                        .mentions(Mentions::with_user_ids([own_user_id.clone()]))
                        .event_id(event_id!("$mention-muted")),
                ),
            );
        })
        .await;

    let muted_mention = owner
        .decide_observed(request(ROOM_ID, Some("$mention-muted")))
        .await
        .expect("muted mention decides");
    assert_eq!(muted_mention.decision, "suppress");
    assert_eq!(
        muted_mention.reason.as_deref(),
        Some("push-rules-no-notify"),
        "mute is a push-rule outcome, not a renderer mode"
    );
    assert!(!muted_mention.highlight && !muted_mention.sound);

    // Rules with `notify` but no tweaks still show without highlight/sound.
    let mut loud = Ruleset::server_default(&own_user_id);
    loud.insert(
        NewPushRule::Room(NewSimplePushRule::new(
            room_id.clone(),
            vec![Action::Notify],
        )),
        None,
        None,
    )
    .unwrap();
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_custom_global_account_data(push_rules_account_data(&loud));
            builder.add_joined_room(
                JoinedRoomBuilder::new(&room_id).add_timeline_event(
                    f.text_msg("all messages again")
                        .sender(*BOB)
                        .event_id(event_id!("$plain-all")),
                ),
            );
        })
        .await;
    let all = owner
        .decide_observed(request(ROOM_ID, Some("$plain-all")))
        .await
        .expect("all-messages decides");
    assert_eq!(all.decision, "show");
    assert!(!all.highlight && !all.sound);
}

#[tokio::test]
async fn observations_outside_the_synced_state_fail_closed_or_fetch_once() {
    let (server, client, f, own_user_id) = synced_group_room().await;
    let owner = NativeNotificationDecisionOwner::new(&client, 7).expect("owner binds session");

    let no_event = owner
        .decide_observed(request(ROOM_ID, None))
        .await
        .expect_err("message decisions require an event id");
    assert_eq!(no_event.diagnostic_id(), "v-notify.event-id-required");

    let bad_room = owner
        .decide_observed(request("not-a-room", Some("$plain")))
        .await
        .expect_err("malformed room ids fail closed");
    assert_eq!(bad_room.diagnostic_id(), "v-notify.invalid-room-id");

    let unknown_room = owner
        .decide_observed(request("!elsewhere:example.org", Some("$plain")))
        .await
        .expect_err("rooms the session has not synced fail closed");
    assert_eq!(unknown_room.diagnostic_id(), "v-notify.room-unknown");
    assert!(
        !format!("{unknown_room:?}").contains("elsewhere"),
        "diagnostics never echo identifiers"
    );

    // An event the cache has not seen is fetched once through the
    // authenticated `/event` route and evaluated with the current push
    // context, so a decision that raced the cache still has one owner.
    let fetched_id = event_id!("$fetched-mention");
    server
        .mock_room_event()
        .room(room_id!("!push-rules:example.org"))
        .match_event_id()
        .ok(f
            .text_msg("fetched mention")
            .sender(*BOB)
            .mentions(Mentions::with_user_ids([own_user_id.clone()]))
            .event_id(fetched_id)
            .into_event())
        .mock_once()
        .mount()
        .await;
    let fetched = owner
        .decide_observed(request(ROOM_ID, Some(fetched_id.as_str())))
        .await
        .expect("fetched event decides");
    assert_eq!(fetched.decision, "show");
    assert!(
        fetched.highlight,
        "fetched events use the same SDK evaluation"
    );

    // Nothing served for this id: the decision fails closed and the renderer
    // resubmits on its next scan instead of notifying blind.
    let missing = owner
        .decide_observed(request(ROOM_ID, Some("$never-existed")))
        .await
        .expect_err("unfetchable events fail closed");
    assert_eq!(missing.diagnostic_id(), "v-notify.event-unavailable");
}
