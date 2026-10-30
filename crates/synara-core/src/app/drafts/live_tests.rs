//! `DraftSyncOwner` against a mock homeserver: what reaches the server is
//! sealed, a peer's synced draft resolves, and nothing syncs without the key.

use std::sync::Arc;

use matrix_sdk::{
    ruma::{events::AnyRoomAccountDataEvent, room_id, serde::Raw, RoomId},
    test_utils::mocks::MatrixMockServer,
    Client,
};
use matrix_sdk_test::JoinedRoomBuilder;
use serde_json::{json, value::to_raw_value};
use wiremock::{
    matchers::{method, path_regex},
    Mock, ResponseTemplate,
};

use super::super::{
    open_envelope, seal_payload, DraftEnvelope, DraftsKey, SynaraDraftSlot, SynaraDraftsPayload,
    DRAFTS_EVENT_TYPE, DRAFTS_KEY_CACHE_KEY, DRAFTS_PAYLOAD_VERSION, MAIN_DRAFT_SLOT,
};
use super::{DraftSyncOwner, NativeDraftDecision, NativeLocalDraftState};

const DRAFTS_PATH: &str = r"^/_matrix/client/v3/user/.*/rooms/.*/account_data/in\.synara\.draft$";

async fn give_key(client: &Client) -> DraftsKey {
    let key = DraftsKey::generate();
    client
        .state_store()
        .set_custom_value_no_read(DRAFTS_KEY_CACHE_KEY, key.as_bytes().to_vec())
        .await
        .unwrap();
    key
}

fn draft_event(envelope: &DraftEnvelope) -> Raw<AnyRoomAccountDataEvent> {
    Raw::from_json(
        to_raw_value(&json!({ "type": DRAFTS_EVENT_TYPE, "content": envelope })).unwrap(),
    )
}

fn peer_payload(markdown: &str, updated_at: u64) -> SynaraDraftsPayload {
    let mut payload = SynaraDraftsPayload {
        v: DRAFTS_PAYLOAD_VERSION,
        ..SynaraDraftsPayload::default()
    };
    payload.slots.insert(
        MAIN_DRAFT_SLOT.to_owned(),
        SynaraDraftSlot {
            updated_at,
            device_id: "PEERDEVICE".to_owned(),
            markdown: Some(markdown.to_owned()),
            ..SynaraDraftSlot::default()
        },
    );
    payload
}

async fn joined_client(server: &MatrixMockServer, room: &RoomId) -> Client {
    let client = server.client_builder().build().await;
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_joined_room(JoinedRoomBuilder::new(room));
        })
        .await;
    client
}

#[tokio::test]
async fn a_flushed_save_reaches_the_server_sealed() {
    let server = MatrixMockServer::new().await;
    let room = room_id!("!drafts:example.org");
    let client = joined_client(&server, room).await;
    let key = give_key(&client).await;
    Mock::given(method("GET"))
        .and(path_regex(DRAFTS_PATH))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "errcode": "M_NOT_FOUND",
            "error": "Account data not found"
        })))
        .mount(server.server())
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(DRAFTS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(server.server())
        .await;
    let owner = DraftSyncOwner::start(&client, Arc::new(|_| {}), 1).unwrap();

    let stamp = owner
        .save(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            Some("secret plan".to_owned()),
            None,
            Some("$reply:example.org".to_owned()),
            None,
            true,
        )
        .await
        .expect("save");

    let put = server
        .server()
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| request.method.as_str() == "PUT")
        .expect("a PUT");
    let body = String::from_utf8(put.body).unwrap();
    assert!(
        !body.contains("secret plan"),
        "the server must see ciphertext only"
    );
    let envelope: DraftEnvelope = serde_json::from_str(&body).unwrap();
    let user_id = client.user_id().unwrap();
    let payload = open_envelope(&key, user_id.as_str(), room.as_str(), &envelope).unwrap();
    let slot = &payload.slots[MAIN_DRAFT_SLOT];
    assert_eq!(slot.markdown.as_deref(), Some("secret plan"));
    assert_eq!(slot.reply_to.as_deref(), Some("$reply:example.org"));
    assert_eq!(slot.updated_at, stamp);
    assert_eq!(slot.device_id, client.device_id().unwrap().as_str());
}

#[tokio::test]
async fn a_peers_synced_draft_applies_when_idle_and_prompts_while_typing() {
    let server = MatrixMockServer::new().await;
    let room = room_id!("!drafts:example.org");
    let client = joined_client(&server, room).await;
    let key = give_key(&client).await;
    let owner = DraftSyncOwner::start(&client, Arc::new(|_| {}), 1).unwrap();
    let user_id = client.user_id().unwrap().to_owned();
    let envelope = seal_payload(
        &key,
        user_id.as_str(),
        room.as_str(),
        &peer_payload("from phone", 5_000),
    )
    .unwrap();
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_joined_room(
                JoinedRoomBuilder::new(room).add_account_data(draft_event(&envelope)),
            );
        })
        .await;

    let idle = owner
        .resolve(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            NativeLocalDraftState {
                updated_at: Some(1_000),
                dirty: false,
                markdown: Some("older".to_owned()),
            },
            false,
        )
        .await
        .unwrap();
    assert_eq!(idle.decision, NativeDraftDecision::Apply);
    assert_eq!(idle.draft.unwrap().markdown.as_deref(), Some("from phone"));

    let typing = owner
        .resolve(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            NativeLocalDraftState {
                updated_at: Some(1_000),
                dirty: true,
                markdown: Some("my own words".to_owned()),
            },
            false,
        )
        .await
        .unwrap();
    assert_eq!(typing.decision, NativeDraftDecision::Prompt);

    let newer_here = owner
        .resolve(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            NativeLocalDraftState {
                updated_at: Some(9_000),
                dirty: false,
                markdown: Some("newer".to_owned()),
            },
            false,
        )
        .await
        .unwrap();
    assert_eq!(newer_here.decision, NativeDraftDecision::Keep);
}

#[tokio::test]
async fn a_draft_sealed_under_another_key_reads_as_absent() {
    let server = MatrixMockServer::new().await;
    let room = room_id!("!drafts:example.org");
    let client = joined_client(&server, room).await;
    let _ours = give_key(&client).await;
    let theirs = DraftsKey::generate();
    let owner = DraftSyncOwner::start(&client, Arc::new(|_| {}), 1).unwrap();
    let user_id = client.user_id().unwrap().to_owned();
    let envelope = seal_payload(
        &theirs,
        user_id.as_str(),
        room.as_str(),
        &peer_payload("old key", 5_000),
    )
    .unwrap();
    server
        .mock_sync()
        .ok_and_run(&client, |builder| {
            builder.add_joined_room(
                JoinedRoomBuilder::new(room).add_account_data(draft_event(&envelope)),
            );
        })
        .await;

    let resolution = owner
        .resolve(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            NativeLocalDraftState::default(),
            false,
        )
        .await
        .unwrap();
    assert_eq!(resolution.decision, NativeDraftDecision::Keep);
}

#[tokio::test]
async fn without_the_key_nothing_is_written_and_nothing_resolves() {
    let server = MatrixMockServer::new().await;
    let room = room_id!("!drafts:example.org");
    let client = joined_client(&server, room).await;
    let owner = DraftSyncOwner::start(&client, Arc::new(|_| {}), 1).unwrap();

    let error = owner
        .save(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            Some("x".into()),
            None,
            None,
            None,
            true,
        )
        .await
        .unwrap_err();
    assert_eq!(error, "drafts-key-missing");
    let resolution = owner
        .resolve(
            room.as_str(),
            MAIN_DRAFT_SLOT,
            NativeLocalDraftState::default(),
            false,
        )
        .await
        .unwrap();
    assert_eq!(resolution.decision, NativeDraftDecision::Keep);
    assert!(server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .all(|request| request.method.as_str() != "PUT"));
}

#[tokio::test]
async fn disabled_sync_rejects_writes_and_reports_disabled() {
    let server = MatrixMockServer::new().await;
    let room = room_id!("!drafts:example.org");
    let client = joined_client(&server, room).await;
    give_key(&client).await;
    let owner = DraftSyncOwner::start(&client, Arc::new(|_| {}), 1).unwrap();
    owner.set_enabled(false);

    assert_eq!(
        owner
            .save(
                room.as_str(),
                MAIN_DRAFT_SLOT,
                Some("x".into()),
                None,
                None,
                None,
                true
            )
            .await
            .unwrap_err(),
        "drafts-sync-disabled"
    );
    assert_eq!(
        owner.status().await.key_state,
        super::NativeDraftKeyState::Disabled
    );
}

#[tokio::test]
async fn invalid_rooms_and_slots_are_rejected_before_any_io() {
    let server = MatrixMockServer::new().await;
    let room = room_id!("!drafts:example.org");
    let client = joined_client(&server, room).await;
    give_key(&client).await;
    let owner = DraftSyncOwner::start(&client, Arc::new(|_| {}), 1).unwrap();

    assert_eq!(
        owner
            .save("not-a-room", MAIN_DRAFT_SLOT, None, None, None, None, true)
            .await
            .unwrap_err(),
        "drafts-invalid-room"
    );
    assert_eq!(
        owner
            .save(room.as_str(), "thread:nope", None, None, None, None, true)
            .await
            .unwrap_err(),
        "drafts-invalid-slot"
    );
    assert_eq!(
        owner
            .save(
                room.as_str(),
                MAIN_DRAFT_SLOT,
                None,
                None,
                Some("bad".into()),
                None,
                true
            )
            .await
            .unwrap_err(),
        "drafts-invalid-event-id"
    );
}
