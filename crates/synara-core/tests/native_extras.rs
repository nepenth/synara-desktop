//! Raw account data, room aliases, mutual rooms, room upgrade and bulk
//! redaction against a mock homeserver.

use std::{collections::BTreeMap, sync::Arc};

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEvent, AnyRoomAccountDataEvent},
        room_id,
        serde::Raw,
        user_id, Int, RoomVersionId,
    },
    test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate},
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
use serde_json::{json, value::to_raw_value};
use synara_core::app::account_data::{NativeImagePackOwner, RawAccountDataError};
use synara_core::app::room_profile::{
    NativeBulkRedactRequest, NativeRoomAliasAvailability, NativeRoomJoinRuleOwner,
};
use wiremock::{
    matchers::{method, path, path_regex},
    Mock, ResponseTemplate,
};

fn global_event(event_type: &str, content: serde_json::Value) -> Raw<AnyGlobalAccountDataEvent> {
    Raw::from_json(to_raw_value(&json!({ "type": event_type, "content": content })).unwrap())
}

fn room_event(event_type: &str, content: serde_json::Value) -> Raw<AnyRoomAccountDataEvent> {
    Raw::from_json(to_raw_value(&json!({ "type": event_type, "content": content })).unwrap())
}

#[tokio::test]
async fn raw_account_data_lists_reads_and_writes_without_secret_types() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let owner = NativeImagePackOwner::start(&client, Arc::new(|_| {}), 3).expect("owner");
    let room_id = room_id!("!layout:example.org");

    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder
                .add_global_account_data(global_event(
                    "in.synara.spaces",
                    json!({ "sidebar": ["!space:example.org"] }),
                ))
                .add_global_account_data(global_event(
                    "m.secret_storage.default_key",
                    json!({ "key": "abc" }),
                ))
                .add_joined_room(
                    JoinedRoomBuilder::new(room_id)
                        .add_account_data(room_event("org.example.note", json!({ "n": 1 }))),
                );
        })
        .await;

    let global = owner.account_data_types(None).expect("types");
    assert_eq!(global.session_generation, 3);
    assert!(global.types.contains(&"in.synara.spaces".to_owned()));
    assert!(!global
        .types
        .iter()
        .any(|kind| kind.starts_with("m.secret_storage")));

    let room = owner
        .account_data_types(Some(room_id.as_str()))
        .expect("room types");
    assert_eq!(room.types, vec!["org.example.note".to_owned()]);

    let spaces = owner
        .account_data_get("in.synara.spaces", None)
        .await
        .expect("read");
    assert_eq!(
        spaces.content,
        Some(json!({ "sidebar": ["!space:example.org"] }))
    );
    let note = owner
        .account_data_get("org.example.note", Some(room_id.as_str()))
        .await
        .expect("room read");
    assert_eq!(note.content, Some(json!({ "n": 1 })));
    let missing = owner
        .account_data_get("io.element.recent_emoji", None)
        .await
        .expect("missing read");
    assert_eq!(missing.content, None);

    assert_eq!(
        owner
            .account_data_get("m.secret_storage.default_key", None)
            .await
            .unwrap_err(),
        RawAccountDataError::SecretBearingType
    );

    Mock::given(method("PUT"))
        .and(path_regex(
            r"^/_matrix/client/v3/user/.*/account_data/io\.element\.recent_emoji$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(server.server())
        .await;
    let written = owner
        .account_data_set(
            "io.element.recent_emoji",
            None,
            json!({ "recent_emoji": [["👍", 2]] }),
        )
        .await
        .expect("write");
    assert_eq!(
        written.content,
        Some(json!({ "recent_emoji": [["👍", 2]] }))
    );
    assert!(owner
        .account_data_types(None)
        .unwrap()
        .types
        .contains(&"io.element.recent_emoji".to_owned()));

    assert_eq!(
        owner
            .account_data_set("m.cross_signing.master", None, json!({}))
            .await
            .unwrap_err(),
        RawAccountDataError::SecretBearingType
    );
    assert_eq!(
        owner
            .account_data_set("org.example.list", None, json!([1, 2]))
            .await
            .unwrap_err(),
        RawAccountDataError::InvalidContent
    );
}

#[tokio::test]
async fn alias_check_is_available_only_on_not_found() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");

    Mock::given(method("GET"))
        .and(path_regex(
            r"^/_matrix/client/v3/directory/room/(%23|#)free(%3A|:)example\.org$",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "errcode": "M_NOT_FOUND", "error": "Room alias not found"
        })))
        .mount(server.server())
        .await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/_matrix/client/v3/directory/room/(%23|#)taken(%3A|:)example\.org$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "room_id": "!taken:example.org", "servers": ["example.org"]
        })))
        .mount(server.server())
        .await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/_matrix/client/v3/directory/room/(%23|#)broken(%3A|:)example\.org$",
        ))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "errcode": "M_UNKNOWN", "error": "boom"
        })))
        .mount(server.server())
        .await;

    let free = owner.alias_check("#free:example.org").await.expect("free");
    assert_eq!(free.availability, NativeRoomAliasAvailability::Available);
    let taken = owner
        .alias_check("#taken:example.org")
        .await
        .expect("taken");
    assert_eq!(taken.availability, NativeRoomAliasAvailability::Taken);
    assert_eq!(
        owner.alias_check("#broken:example.org").await.unwrap_err(),
        "v-rooms-alias-check-failed"
    );
    assert_eq!(
        owner.alias_check("not an alias").await.unwrap_err(),
        "v-rooms-alias-invalid"
    );
}

#[tokio::test]
async fn local_aliases_create_and_delete_go_to_the_directory() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");

    Mock::given(method("GET"))
        .and(path("/_matrix/client/v3/rooms/!room:example.org/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "aliases": ["#b:example.org", "#a:example.org"]
        })))
        .mount(server.server())
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(
            r"^/_matrix/client/v3/directory/room/(%23|#)new(%3A|:)example\.org$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(server.server())
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(
            r"^/_matrix/client/v3/directory/room/(%23|#)dup(%3A|:)example\.org$",
        ))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "errcode": "M_UNKNOWN", "error": "Room alias already exists"
        })))
        .mount(server.server())
        .await;
    Mock::given(method("DELETE"))
        .and(path_regex(
            r"^/_matrix/client/v3/directory/room/(%23|#)old(%3A|:)example\.org$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(server.server())
        .await;

    let aliases = owner
        .local_aliases("!room:example.org")
        .await
        .expect("aliases");
    assert_eq!(aliases.aliases, vec!["#a:example.org", "#b:example.org"]);
    owner
        .alias_create("#new:example.org", "!room:example.org")
        .await
        .expect("create");
    assert_eq!(
        owner
            .alias_create("#dup:example.org", "!room:example.org")
            .await
            .unwrap_err(),
        "v-rooms-alias-taken"
    );
    owner
        .alias_delete("#old:example.org")
        .await
        .expect("delete");
}

#[tokio::test]
async fn mutual_rooms_are_joined_rooms_where_the_user_is_joined() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");
    let bob = user_id!("@bob:example.org");
    let shared = room_id!("!shared:example.org");
    let other = room_id!("!other:example.org");
    let factory = EventFactory::new().sender(bob);

    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder
                .add_joined_room(
                    JoinedRoomBuilder::new(shared)
                        .add_state_event(factory.member(bob).room(shared)),
                )
                .add_joined_room(JoinedRoomBuilder::new(other));
        })
        .await;

    let mutual = owner.mutual_rooms(bob.as_str()).await.expect("mutual");
    assert_eq!(mutual.room_ids, vec![shared.to_string()]);
    assert_eq!(
        owner.mutual_rooms("bob").await.unwrap_err(),
        "v-rooms-mutual-invalid-user"
    );
}

#[tokio::test]
async fn room_upgrade_requires_tombstone_power_and_returns_the_replacement() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");
    let me = client.user_id().unwrap().to_owned();
    let admin_room = room_id!("!admin:example.org");
    let member_room = room_id!("!member:example.org");
    let factory = EventFactory::new().sender(&me);
    let mut admin = BTreeMap::from([(me.clone(), Int::from(100))]);

    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder
                .add_joined_room(
                    JoinedRoomBuilder::new(admin_room)
                        .add_state_event(factory.create(&me, RoomVersionId::V10).room(admin_room))
                        .add_state_event(factory.power_levels(&mut admin).room(admin_room)),
                )
                .add_joined_room(
                    JoinedRoomBuilder::new(member_room)
                        .add_state_event(
                            factory
                                .create(user_id!("@other:example.org"), RoomVersionId::V10)
                                .sender(user_id!("@other:example.org"))
                                .room(member_room),
                        )
                        .add_state_event(
                            factory
                                .default_power_levels()
                                .sender(user_id!("@other:example.org"))
                                .room(member_room),
                        ),
                );
        })
        .await;

    let replacement = room_id!("!new:example.org");
    server
        .mock_upgrade_room()
        .ok_with(replacement)
        .expect(1)
        .mount()
        .await;

    let upgraded = owner
        .upgrade_room(admin_room.as_str(), "11", &[])
        .await
        .expect("upgrade");
    assert_eq!(upgraded.replacement_room_id, replacement.to_string());
    assert_eq!(
        owner
            .upgrade_room(member_room.as_str(), "11", &[])
            .await
            .unwrap_err(),
        "v-rooms-upgrade-forbidden"
    );
    assert_eq!(
        owner
            .upgrade_room(admin_room.as_str(), "not a version!", &[])
            .await
            .unwrap_err(),
        "v-rooms-upgrade-invalid-version"
    );
    assert_eq!(
        owner
            .upgrade_room(admin_room.as_str(), "12", &["not-a-user".to_owned()])
            .await
            .unwrap_err(),
        "v-rooms-upgrade-invalid-creators"
    );
}

#[tokio::test]
async fn bulk_redact_removes_recent_messages_from_listed_users_only() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");
    let room_id = room_id!("!spam:example.org");
    server.sync_joined_room(&client, room_id).await;

    let spammer = user_id!("@spam:example.org");
    let friend = user_id!("@friend:example.org");
    let factory = EventFactory::new().room(room_id);
    let now: u64 = matrix_sdk::ruma::MilliSecondsSinceUnixEpoch::now()
        .get()
        .into();
    let recent = matrix_sdk::ruma::MilliSecondsSinceUnixEpoch(
        matrix_sdk::ruma::UInt::try_from(now - 1_000).unwrap(),
    );
    let old = matrix_sdk::ruma::MilliSecondsSinceUnixEpoch(matrix_sdk::ruma::UInt::from(1_000u32));

    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().events(vec![
            factory
                .text_msg("spam 1")
                .sender(spammer)
                .event_id(matrix_sdk::ruma::event_id!("$spam1:example.org"))
                .server_ts(recent),
            factory
                .text_msg("hello")
                .sender(friend)
                .event_id(matrix_sdk::ruma::event_id!("$friend:example.org"))
                .server_ts(recent),
            factory
                .text_msg("spam old")
                .sender(spammer)
                .event_id(matrix_sdk::ruma::event_id!("$old:example.org"))
                .server_ts(old),
        ]))
        .mock_once()
        .mount()
        .await;
    server
        .mock_room_redact()
        .ok(matrix_sdk::ruma::event_id!("$redaction:example.org"))
        .expect(1)
        .mount()
        .await;

    let result = owner
        .bulk_redact(NativeBulkRedactRequest {
            room_id: room_id.to_string(),
            user_ids: vec![spammer.to_string()],
            since_ts: now - 10_000,
            event_types: Vec::new(),
            reason: Some("spam".to_owned()),
        })
        .await
        .expect("bulk redact");
    assert_eq!(result.redacted, 1);
    assert_eq!(result.failed, 0);
    assert_eq!(result.scanned, 3);
    assert!(!result.truncated);

    assert_eq!(
        owner
            .bulk_redact(NativeBulkRedactRequest {
                room_id: room_id.to_string(),
                user_ids: Vec::new(),
                since_ts: 0,
                event_types: Vec::new(),
                reason: None,
            })
            .await
            .unwrap_err(),
        "v-rooms-bulk-redact-invalid-users"
    );
}
