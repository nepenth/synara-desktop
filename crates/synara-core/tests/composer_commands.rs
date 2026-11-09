//! Slash commands from the shared composer core against a mock homeserver:
//! the room's real power levels filter suggestions and refuse commands, and
//! commands write through the same room owner the shells' buttons use.

use std::{collections::BTreeMap, sync::Arc};

use matrix_sdk::{
    ruma::{
        event_id,
        events::{room::member::RoomMemberEvent, StateEventType},
        room_id,
        serde::Raw,
        user_id, Int, RoomVersionId,
    },
    test_utils::mocks::MatrixMockServer,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
use serde_json::{json, Value as JsonValue};
use synara_core::app::composer::{
    command_suggestions, run_command, CommandContext, CommandOutcome, ComposerErrorKind,
    ComposerMember, ComposerOps, OpResult, RedactSummary,
};
use synara_core::app::room_profile::NativeRoomJoinRuleOwner;

/// Composer operations over the room owner; what these tests do not use fails.
struct OwnerOps(NativeRoomJoinRuleOwner);

const UNUSED: &str = "composer-test-unused-op";

fn member_event(room_id: &str, user_id: &str, name: &str) -> Raw<RoomMemberEvent> {
    Raw::from_json_string(
        json!({
            "type": "m.room.member",
            "room_id": room_id,
            "state_key": user_id,
            "sender": user_id,
            "event_id": format!("$member-{name}"),
            "origin_server_ts": 1,
            "content": { "membership": "join", "displayname": name },
        })
        .to_string(),
    )
    .expect("member event")
}

impl ComposerOps for OwnerOps {
    fn own_user_id(&self) -> Option<String> {
        self.0.own_user_id()
    }
    fn now_ms(&self) -> u64 {
        0
    }
    async fn context(&self, room_id: &str) -> OpResult<CommandContext> {
        Ok(self.0.composer_context(room_id).await?)
    }
    async fn members(&self, room_id: &str) -> OpResult<Vec<ComposerMember>> {
        let snapshot = self.0.members_snapshot(room_id).await?;
        Ok(snapshot
            .members
            .into_iter()
            .map(|member| ComposerMember {
                user_id: member.user_id,
                display_name: member.display_name,
                membership: member.membership.as_str().to_owned(),
            })
            .collect())
    }
    async fn invite(&self, _: &str, _: &str, _: Option<String>) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn kick(&self, room_id: &str, user_id: &str, reason: Option<String>) -> OpResult<()> {
        Ok(self.0.kick(room_id, user_id, reason).await?)
    }
    async fn ban(&self, room_id: &str, user_id: &str, reason: Option<String>) -> OpResult<()> {
        Ok(self.0.ban(room_id, user_id, reason).await?)
    }
    async fn unban(&self, _: &str, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn join(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn leave(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn ignore(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn unignore(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn set_room_name(&self, room_id: &str, name: &str) -> OpResult<()> {
        self.0
            .set_name(room_id, name)
            .await
            .map(drop)
            .map_err(Into::into)
    }
    async fn set_room_topic(&self, room_id: &str, topic: &str) -> OpResult<()> {
        self.0
            .set_topic(room_id, topic)
            .await
            .map(drop)
            .map_err(Into::into)
    }
    async fn set_display_name(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn set_avatar(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn state_content(
        &self,
        room_id: &str,
        kind: &str,
        key: &str,
    ) -> OpResult<Option<JsonValue>> {
        Ok(self.0.state_content(room_id, kind, key).await?)
    }
    async fn send_state(
        &self,
        room_id: &str,
        kind: &str,
        key: &str,
        content: JsonValue,
    ) -> OpResult<()> {
        self.0
            .send_state_event(room_id, kind, key, content)
            .await
            .map(drop)
            .map_err(Into::into)
    }
    async fn set_power_level(&self, room_id: &str, user_id: &str, level: i64) -> OpResult<()> {
        Ok(self.0.set_power_level(room_id, user_id, level).await?)
    }
    async fn dm_room(&self, user_id: &str) -> OpResult<Option<String>> {
        Ok(self.0.dm_room_for(user_id)?)
    }
    async fn create_dm(&self, _: &[String]) -> OpResult<String> {
        Err(UNUSED.into())
    }
    async fn mark_dm(&self, _: &str, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn unmark_dm(&self, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn send_text(&self, _: &str, _: &str) -> OpResult<()> {
        Err(UNUSED.into())
    }
    async fn bulk_redact(
        &self,
        _: &str,
        _: &[String],
        _: u64,
        _: &[String],
        _: Option<String>,
    ) -> OpResult<RedactSummary> {
        Err(UNUSED.into())
    }
    async fn upgrade_room(&self, room_id: &str, version: &str) -> OpResult<String> {
        Ok(self
            .0
            .upgrade_room(room_id, version, &[])
            .await?
            .replacement_room_id)
    }
    async fn event_at(&self, _: &str, _: u64) -> OpResult<Option<String>> {
        Err(UNUSED.into())
    }
}

#[tokio::test]
async fn composer_commands_follow_the_rooms_power_levels() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let me = client.user_id().unwrap().to_owned();
    let other = user_id!("@other:example.org");
    let admin_room = room_id!("!admin:example.org");
    let member_room = room_id!("!member:example.org");
    let factory = EventFactory::new().sender(&me);
    let mut admin_levels = BTreeMap::from([(me.clone(), Int::from(100))]);

    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder
                .add_joined_room(
                    JoinedRoomBuilder::new(admin_room)
                        .add_state_event(factory.create(&me, RoomVersionId::V10).room(admin_room))
                        .add_state_event(factory.power_levels(&mut admin_levels).room(admin_room))
                        .add_state_event(factory.member(&me).display_name("Old").room(admin_room))
                        .add_state_event(
                            factory
                                .member(other)
                                .display_name("Puck")
                                .sender(other)
                                .room(admin_room),
                        ),
                )
                .add_joined_room(
                    JoinedRoomBuilder::new(member_room)
                        .add_state_event(
                            factory
                                .create(other, RoomVersionId::V10)
                                .sender(other)
                                .room(member_room),
                        )
                        .add_state_event(
                            factory
                                .default_power_levels()
                                .sender(other)
                                .room(member_room),
                        ),
                );
        })
        .await;
    server.mock_room_state_encryption().plain().mount().await;

    let ops =
        OwnerOps(NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner"));

    // A plain member is not offered moderation or room settings, and /ban is refused.
    let member_context = ops
        .0
        .composer_context(member_room.as_str())
        .await
        .expect("context");
    let offered: Vec<String> = command_suggestions("", &member_context)
        .into_iter()
        .map(|suggestion| suggestion.name)
        .collect();
    assert!(offered.contains(&"me".to_owned()));
    for hidden in ["ban", "kick", "topic", "op", "upgraderoom"] {
        assert!(!offered.contains(&hidden.to_owned()), "{hidden}");
    }
    let refused = run_command(&ops, member_room.as_str(), "/ban @other:example.org", true)
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ComposerErrorKind::Permission);

    // The admin sets the topic through the room owner.
    server
        .mock_room_send_state()
        .for_type(StateEventType::RoomTopic)
        .ok(event_id!("$topic"))
        .mock_once()
        .mount()
        .await;
    assert!(matches!(
        run_command(&ops, admin_room.as_str(), "/topic Weekly planning", false).await,
        Ok(CommandOutcome::Done { .. })
    ));

    // /myroomnick keeps the member event's other fields.
    server
        .mock_room_send_state()
        .for_type(StateEventType::RoomMember)
        .for_key(me.to_string())
        .body_matches_partial_json(json!({ "membership": "join", "displayname": "New" }))
        .ok(event_id!("$nick"))
        .mock_once()
        .mount()
        .await;
    assert!(matches!(
        run_command(&ops, admin_room.as_str(), "/myroomnick New", false).await,
        Ok(CommandOutcome::Done { .. })
    ));

    // /whois finds a member by name.
    server
        .mock_get_members()
        .ok(vec![
            member_event(admin_room.as_str(), me.as_str(), "New"),
            member_event(admin_room.as_str(), other.as_str(), "Puck"),
        ])
        .mount()
        .await;
    assert_eq!(
        run_command(&ops, admin_room.as_str(), "/whois Puck", false).await,
        Ok(CommandOutcome::OpenProfile {
            user_id: other.to_string()
        })
    );

    // /upgraderoom asks first, then upgrades and moves to the new room.
    assert!(matches!(
        run_command(&ops, admin_room.as_str(), "/upgraderoom 11", false).await,
        Ok(CommandOutcome::NeedsConfirmation { .. })
    ));
    let replacement = room_id!("!new:example.org");
    server
        .mock_upgrade_room()
        .ok_with(replacement)
        .expect(1)
        .mount()
        .await;
    assert_eq!(
        run_command(&ops, admin_room.as_str(), "/upgraderoom 11", true).await,
        Ok(CommandOutcome::Navigate {
            room_id: replacement.to_string(),
            event_id: None
        })
    );
}
