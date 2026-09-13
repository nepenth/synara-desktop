//! The real SDK supplies unopened room state, reaction aggregation and redaction
//! diffs. Only its homeserver HTTP transport is deterministic test data.
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, LeftRoomBuilder, BOB};
use ruma::{event_id, room_id, RoomVersionId};
use std::{sync::Arc, time::Duration};
use synara_core::app::timeline::{
    NativeAgentApprovalInboxSnapshot, NativeAgentApprovalInboxStatus, NativeTimelineOwner,
};

const PROMPT: &str =
    "⚠️ Approval required: dangerous command\nCommand: rm example\nReact with ✅, ♾️, or ❌.";

async fn wait_for(
    owner: &NativeTimelineOwner,
    accept: impl Fn(&NativeAgentApprovalInboxSnapshot) -> bool,
) -> NativeAgentApprovalInboxSnapshot {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = owner
                .agent_approvals_list_with_discovery(true)
                .await
                .unwrap();
            if accept(&snapshot) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("approval SDK observers must settle")
}

#[tokio::test]
async fn discovers_unopened_rooms_and_tracks_own_reactions_and_redactions() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let own_user = client.user_id().unwrap().to_owned();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let room_a = room_id!("!approval-a:example.org");
    let room_b = room_id!("!approval-b:example.org");
    let prompt_a = event_id!("$approval-a");
    let prompt_b = event_id!("$approval-b");
    for (room, prompt, older) in [
        (room_a, prompt_a, event_id!("$older-a")),
        (room_b, prompt_b, event_id!("$older-b")),
    ] {
        let f = EventFactory::new().room(room);
        server
            .sync_room(
                &client,
                JoinedRoomBuilder::new(room)
                    .add_state_event(f.create(&own_user, RoomVersionId::V11))
                    .add_timeline_event(
                        f.text_msg("earlier conversation")
                            .sender(*BOB)
                            .event_id(older)
                            .server_ts(now - 360_000),
                    )
                    .add_timeline_event(
                        f.text_msg(PROMPT)
                            .sender(*BOB)
                            .event_id(prompt)
                            .server_ts(now),
                    )
                    .add_timeline_event(f.reaction(prompt, "✅").sender(*BOB).server_ts(now)),
            )
            .await;
    }
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 7);
    // No timeline open/view command precedes discovery.
    let initial = owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    assert!(initial.loading);
    let ready = wait_for(&owner, |s| !s.loading && s.items.len() == 2).await;
    assert_eq!(ready.session_generation, 7);
    assert!(!ready.incomplete);
    assert!(ready
        .items
        .iter()
        .all(|item| item.status == NativeAgentApprovalInboxStatus::Pending));
    assert!(ready
        .items
        .iter()
        .any(|item| item.room_id == room_a.as_str()));
    assert!(ready
        .items
        .iter()
        .any(|item| item.room_id == room_b.as_str()));

    let f = EventFactory::new().room(room_a);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_a).add_timeline_event(
                f.reaction(prompt_a, "❌")
                    .sender(&own_user)
                    .server_ts(now + 1),
            ),
        )
        .await;
    wait_for(&owner, |s| {
        s.items.iter().any(|item| {
            item.event_id == prompt_a.as_str()
                && item.status == NativeAgentApprovalInboxStatus::Decided
        })
    })
    .await;

    let f = EventFactory::new().room(room_b);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_b)
                .add_timeline_event(f.redaction(prompt_b).sender(&own_user).server_ts(now + 2)),
        )
        .await;
    let redacted = wait_for(&owner, |s| s.items.len() == 1).await;
    assert_eq!(redacted.items[0].event_id, prompt_a.as_str());
    server
        .sync_room(&client, LeftRoomBuilder::new(room_a))
        .await;
    assert!(owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap()
        .items
        .is_empty());
    let requests = server.server().received_requests().await.unwrap();
    assert!(
        requests.iter().all(|request| {
            !request.url.path().contains("/receipt/")
                && !request.url.path().ends_with("/read_markers")
                && !request.url.path().contains("/send/")
        }),
        "approval discovery must remain read-only"
    );
}

#[tokio::test]
async fn discovers_cached_approvals_beyond_the_sdk_initial_twenty_item_window() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!approval-long:example.org");
    let f = EventFactory::new().room(room_id);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let mut joined = JoinedRoomBuilder::new(room_id)
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
        .add_timeline_event(
            f.text_msg("before the approval window")
                .sender(*BOB)
                .server_ts(now - 360_000),
        )
        .add_timeline_event(
            f.text_msg(PROMPT)
                .sender(*BOB)
                .event_id(event_id!("$outside-initial-tail"))
                .server_ts(now - 60_000),
        );
    for index in 0..80 {
        joined = joined.add_timeline_event(
            f.text_msg(format!("later message {index}"))
                .sender(*BOB)
                .server_ts(now),
        );
    }
    server.sync_room(&client, joined).await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 4);
    let ready = wait_for(&owner, |s| !s.loading && !s.incomplete).await;
    assert_eq!(ready.items.len(), 1);
    assert_eq!(ready.items[0].event_id, "$outside-initial-tail");
}

#[tokio::test]
async fn limited_sync_gap_invalidates_coverage_until_recent_history_is_recovered() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!approval-gap:example.org");
    let f = EventFactory::new().room(room_id);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let older = event_id!("$gap-boundary");
    let prompt = event_id!("$gap-prompt");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(
                    f.text_msg("before the window")
                        .sender(*BOB)
                        .event_id(older)
                        .server_ts(now - 360_000),
                )
                .add_timeline_event(
                    f.text_msg(PROMPT)
                        .sender(*BOB)
                        .event_id(prompt)
                        .server_ts(now - 60_000),
                ),
        )
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 5);
    wait_for(&owner, |s| {
        !s.loading && !s.incomplete && s.items.len() == 1
    })
    .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default()
            .events(vec![
                f.text_msg(PROMPT)
                    .sender(*BOB)
                    .event_id(prompt)
                    .server_ts(now - 60_000)
                    .into_raw_timeline(),
                f.text_msg("before the window")
                    .sender(*BOB)
                    .event_id(older)
                    .server_ts(now - 360_000)
                    .into_raw_timeline(),
            ])
            .with_delay(Duration::from_millis(300)))
        .mount()
        .await;
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .set_timeline_limited()
                .set_timeline_prev_batch("gap-to-recover")
                .add_timeline_event(f.text_msg("after a sync gap").sender(*BOB).server_ts(now)),
        )
        .await;
    let gap = wait_for(&owner, |s| s.incomplete).await;
    assert!(
        gap.items.is_empty(),
        "old coverage cannot survive a cache reset"
    );
    let recovered = wait_for(&owner, |s| !s.incomplete && s.items.len() == 1).await;
    assert_eq!(recovered.items[0].event_id, prompt.as_str());
}

#[tokio::test]
async fn delayed_history_retry_does_not_delay_live_redaction() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!approval-concurrent:example.org");
    let f = EventFactory::new().room(room_id);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let older = event_id!("$concurrent-boundary");
    let latest = event_id!("$concurrent-prompt");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(
                    f.text_msg("before the window")
                        .sender(*BOB)
                        .event_id(older)
                        .server_ts(now - 360_000),
                ),
        )
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 6);
    wait_for(&owner, |s| !s.loading && !s.incomplete).await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default()
            .events(vec![f
                .text_msg("before the window")
                .sender(*BOB)
                .event_id(older)
                .server_ts(now - 360_000)
                .into_raw_timeline()])
            .with_delay(Duration::from_secs(2)))
        .mount()
        .await;
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .set_timeline_limited()
                .set_timeline_prev_batch("slow-gap")
                .add_timeline_event(
                    f.text_msg(PROMPT)
                        .sender(*BOB)
                        .event_id(latest)
                        .server_ts(now),
                ),
        )
        .await;
    wait_for(&owner, |s| {
        s.incomplete && s.items.iter().any(|item| item.event_id == latest.as_str())
    })
    .await;
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if server
                .server()
                .received_requests()
                .await
                .unwrap()
                .iter()
                .any(|request| request.url.path().ends_with("/messages"))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("retry must start before injecting the redaction");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(
                f.redaction(latest)
                    .sender(client.user_id().unwrap())
                    .server_ts(now + 1),
            ),
        )
        .await;
    let redacted = tokio::time::timeout(
        Duration::from_millis(500),
        wait_for(&owner, |s| s.items.is_empty()),
    )
    .await
    .expect("live diffs must apply while the two-second history request is pending");
    assert!(redacted.incomplete, "history is still in flight");
    wait_for(&owner, |s| !s.incomplete).await;
}

#[tokio::test]
async fn incomplete_initial_bootstrap_immediately_continues_past_three_hundred_rows() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!approval-busy:example.org");
    let f = EventFactory::new().room(room_id);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let mut joined = JoinedRoomBuilder::new(room_id)
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
        .add_timeline_event(
            f.text_msg("before the approval window")
                .sender(*BOB)
                .server_ts(now - 360_000),
        )
        .add_timeline_event(
            f.text_msg(PROMPT)
                .sender(*BOB)
                .event_id(event_id!("$outside-first-backfill"))
                .server_ts(now - 60_000),
        );
    for index in 0..400 {
        joined = joined.add_timeline_event(
            f.text_msg(format!("later busy-room message {index}"))
                .sender(*BOB)
                .server_ts(now),
        );
    }
    server.sync_room(&client, joined).await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 8);
    let ready = tokio::time::timeout(
        Duration::from_secs(2),
        wait_for(&owner, |s| !s.loading && !s.incomplete),
    )
    .await
    .expect("initially incomplete discovery must not wait for the 30-second retry tick");
    assert_eq!(ready.items.len(), 1);
    assert_eq!(ready.items[0].event_id, "$outside-first-backfill");
}

#[tokio::test]
async fn encrypted_prompt_is_incomplete_until_native_key_arrival_then_uses_room_authority() {
    use matrix_sdk_crypto::{olm::EncryptionSettings, OlmMachine};
    use ruma::{
        device_id,
        events::room::{
            encrypted::RoomEncryptedEventContent, message::RoomMessageEventContent,
            power_levels::RoomPowerLevelsEventContent,
        },
        UserId,
    };
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().logged_in_with_oauth().build().await;
    client.event_cache().subscribe().unwrap();
    let own_user = client.user_id().unwrap().to_owned();
    let room_id = room_id!("!encrypted-approval:example.org");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let sender = OlmMachine::new(*BOB, device_id!("HERMES")).await;
    sender
        .share_room_key(
            room_id,
            std::iter::empty::<&UserId>(),
            EncryptionSettings::default(),
        )
        .await
        .unwrap();
    let encrypted = sender
        .encrypt_room_event(room_id, RoomMessageEventContent::text_plain(PROMPT))
        .await
        .unwrap();
    let content: RoomEncryptedEventContent =
        serde_json::from_str(encrypted.content.json().get()).unwrap();
    let f = EventFactory::new().room(room_id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user, RoomVersionId::V11))
                .add_state_event(
                    f.event(
                        serde_json::from_value::<RoomPowerLevelsEventContent>(
                            serde_json::json!({"events": {"m.reaction": 0}}),
                        )
                        .unwrap(),
                    )
                    .sender(&own_user)
                    .state_key(""),
                )
                .add_timeline_event(
                    f.text_msg("covered boundary")
                        .sender(*BOB)
                        .server_ts(now - 360_000),
                )
                .add_timeline_event(
                    f.event(content)
                        .sender(*BOB)
                        .event_id(event_id!("$encrypted-approval"))
                        .server_ts(now),
                ),
        )
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 19);
    let badge = owner.agent_approvals_list().await.unwrap();
    assert!(badge.incomplete);
    assert!(
        !badge.loading,
        "encrypted latest-event hint must not silently scan every encrypted room"
    );
    let encrypted = wait_for(&owner, |snapshot| !snapshot.loading).await;
    assert!(encrypted.incomplete);
    assert!(encrypted.items.is_empty());
    let keys = sender
        .store()
        .export_room_keys(|session| session.room_id() == room_id)
        .await
        .unwrap();
    assert!(!keys.is_empty());
    client
        .olm_machine_for_testing()
        .await
        .as_ref()
        .unwrap()
        .store()
        .import_exported_room_keys(keys, |_, _| {})
        .await
        .unwrap();
    let decrypted = wait_for(&owner, |snapshot| snapshot.items.len() == 1).await;
    assert!(!decrypted.incomplete);
    assert_eq!(decrypted.items[0].event_id, "$encrypted-approval");
    assert_eq!(decrypted.items[0].body, PROMPT);
    assert!(decrypted.items[0].can_send_reaction);
    let levels: RoomPowerLevelsEventContent = serde_json::from_value(serde_json::json!({
        "users": {own_user.as_str(): 0}, "events": {"m.reaction": 100}
    }))
    .unwrap();
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.event(levels).sender(&own_user).state_key("")),
        )
        .await;
    let forbidden = owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    assert!(!forbidden.items[0].can_send_reaction);
    assert_eq!(
        forbidden.items[0].status,
        NativeAgentApprovalInboxStatus::Pending
    );
}
