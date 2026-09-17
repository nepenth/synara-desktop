//! MSC4362 encrypted-state product adapters against the pinned SDK mock server.
#![recursion_limit = "256"]

use std::sync::Arc;

use matrix_sdk::{
    encryption::EncryptionSettings,
    ruma::{
        device_id, event_id,
        events::{room::power_levels::RoomPowerLevelsEventContent, StateEventType},
        room_id,
        room_version_rules::AuthorizationRules,
        user_id, RoomVersionId,
    },
    test_utils::mocks::MatrixMockServer,
};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
use serde_json::json;
use synara_core::app::room_profile::NativeRoomJoinRuleOwner;
use synara_core::app::spaces::set_space_child;

async fn state_encrypted_owner(
    server: &MatrixMockServer,
) -> (matrix_sdk::Client, NativeRoomJoinRuleOwner) {
    server.mock_crypto_endpoints_preset().await;
    server
        .mock_room_state_encryption()
        .state_encrypted()
        .mount()
        .await;
    let encryption_settings = EncryptionSettings {
        auto_enable_cross_signing: true,
        ..Default::default()
    };
    let user = user_id!("@alice:localhost");
    let device = device_id!("ALICEDEVICE");
    let client = server
        .client_builder_for_crypto_end_to_end(user, device)
        .on_builder(|builder| {
            builder
                .with_enable_share_history_on_invite(true)
                .with_encryption_settings(encryption_settings)
        })
        .build()
        .await;
    let room_id = room_id!("!test:localhost");
    let factory = EventFactory::new().sender(user).room(room_id);
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_joined_room(
                JoinedRoomBuilder::new(room_id)
                    .add_state_event(factory.create(user, RoomVersionId::V11))
                    .add_state_event(factory.room_encryption_with_state_encryption()),
            );
        })
        .await;
    server
        .mock_get_members()
        .ok(vec![factory.member(user).into_raw()])
        .mock_once()
        .mount()
        .await;
    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");
    (client, owner)
}

#[tokio::test]
async fn state_encrypted_set_name_uses_packed_encrypted_key() {
    let server = MatrixMockServer::new().await;
    let (_client, owner) = state_encrypted_owner(&server).await;
    server
        .mock_room_send_state()
        .for_type(StateEventType::RoomEncrypted)
        .expect_access_token("TOKEN_0")
        .for_key("m.room.name:".to_owned())
        .body_matches_partial_json(json!({
            "algorithm": "m.megolm.v1.aes-sha2",
            "device_id": "ALICEDEVICE",
        }))
        .ok(event_id!("$name"))
        .mock_once()
        .mount()
        .await;
    owner
        .set_name("!test:localhost", "Secret")
        .await
        .expect("native set_name must encrypt through the SDK future");
}

#[tokio::test]
async fn excluded_types_stay_plaintext_in_state_encrypted_rooms() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    server
        .mock_room_state_encryption()
        .state_encrypted()
        .mount()
        .await;
    let room_id = room_id!("!plain:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    assert!(room
        .latest_encryption_state()
        .await
        .unwrap()
        .is_state_encrypted());

    server
        .mock_room_send_state()
        .for_type(StateEventType::RoomPowerLevels)
        .ok(event_id!("$pl"))
        .mock_once()
        .mount()
        .await;
    room.send_state_event(RoomPowerLevelsEventContent::new(&AuthorizationRules::V1))
        .await
        .expect("power levels stay plaintext");

    server.verify_and_reset().await;
    server
        .mock_room_state_encryption()
        .state_encrypted()
        .mount()
        .await;
    server
        .mock_room_send_state()
        .for_type(StateEventType::from("m.space.child"))
        .for_key("!child:example.org".to_owned())
        .ok(event_id!("$child"))
        .mock_once()
        .mount()
        .await;
    set_space_child(
        &client,
        room_id.as_str(),
        "!child:example.org",
        &["example.org".to_owned()],
        None,
        None,
    )
    .await
    .expect("space child stays plaintext");
}

#[tokio::test]
async fn enable_helper_noops_when_already_encrypted_product_opt_in_sends() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    server
        .mock_room_state_encryption()
        .encrypted()
        .mount()
        .await;
    let room_id = room_id!("!e2ee:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    let state = room.latest_encryption_state().await.unwrap();
    assert!(state.is_encrypted());
    assert!(!state.is_state_encrypted());

    room.enable_encryption_with_state_event_encryption()
        .await
        .expect("SDK helper no-ops when already encrypted");

    let owner = NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), 1).expect("owner");
    server
        .mock_room_send_state()
        .for_type(StateEventType::RoomEncryption)
        .body_matches_partial_json(json!({
            "algorithm": "m.megolm.v1.aes-sha2",
            "encrypt_state_events": true,
        }))
        .ok(event_id!("$opt-in"))
        .mock_once()
        .mount()
        .await;
    owner
        .enable_room_encrypted_state(room_id.as_str(), true)
        .await
        .expect("product opt-in must send m.room.encryption");
}

#[test]
fn native_paths_still_call_sdk_send_state_event_futures() {
    let profile = include_str!("../src/app/room_profile/live.rs");
    let spaces = include_str!("../src/app/spaces/live.rs");
    let packs = include_str!("../src/app/account_data/image_packs_live.rs");
    let list = include_str!("../src/app/room_list/live.rs");
    assert!(profile.contains("room.send_state_event_raw(event_type, state_key, content)"));
    assert!(profile.contains("room.set_name(name)"));
    assert!(spaces.contains("room.send_state_event_for_key(&child, content)"));
    assert!(packs.contains("room.send_state_event_raw(ROOM_EMOTES_EVENT_TYPE, state_key, content)"));
    assert!(!profile.contains("force_plaintext"));
    assert!(
        profile.contains("PowerLevelTagsReadbackSource::Store"),
        "tags readback in StateEncrypted rooms must use the decrypted store"
    );
    assert!(
        list.contains("cached_display_name()"),
        "mixed rooms prefer encrypted via the SDK store, not a product plaintext overlay"
    );
}
