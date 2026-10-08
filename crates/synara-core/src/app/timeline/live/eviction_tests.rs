//! Idle live room timelines are bounded: the least recently used one is
//! dropped past [`MAX_LIVE_TIMELINES`], and a room-list Mark as Read does not
//! displace rooms the user actually opened.

use super::*;
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};

fn test_room(index: usize) -> OwnedRoomId {
    OwnedRoomId::try_from(format!("!eviction-{index}:example.org")).expect("valid room id")
}

async fn joined_rooms(server: &MatrixMockServer, client: &Client, count: usize) -> Vec<String> {
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    let mut rooms = Vec::with_capacity(count);
    for index in 0..count {
        let room_id = test_room(index);
        server.sync_joined_room(client, &room_id).await;
        rooms.push(room_id.to_string());
    }
    rooms
}

#[tokio::test]
async fn opening_past_the_cap_evicts_the_least_recently_used_room() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let rooms = joined_rooms(&server, &client, MAX_LIVE_TIMELINES + 2).await;
    let mut registry = NativeTimelineRegistry::new(1);

    for room_id in &rooms[..MAX_LIVE_TIMELINES] {
        registry.open(&client, room_id).await.expect("open room");
    }
    assert_eq!(registry.entries.len(), MAX_LIVE_TIMELINES);

    // Reusing the first room makes the second one the least recently used.
    registry
        .open(&client, &rooms[0])
        .await
        .expect("reopen room");
    registry
        .open(&client, &rooms[MAX_LIVE_TIMELINES])
        .await
        .expect("open room past the cap");
    assert_eq!(registry.entries.len(), MAX_LIVE_TIMELINES);
    assert!(
        registry.entries.contains_key(&rooms[0]),
        "recently used room stays"
    );
    assert!(
        !registry.entries.contains_key(&rooms[1]),
        "least recently used room is evicted"
    );
    assert!(registry.entries.contains_key(&rooms[MAX_LIVE_TIMELINES]));
}

#[tokio::test]
async fn room_list_mark_read_timeline_is_the_first_eviction_candidate() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let rooms = joined_rooms(&server, &client, MAX_LIVE_TIMELINES + 1).await;
    let mut registry = NativeTimelineRegistry::new(1);

    for room_id in &rooms[..MAX_LIVE_TIMELINES - 1] {
        registry.open(&client, room_id).await.expect("open room");
    }
    // A room that was not open before is only borrowed by Mark as Read.
    let borrowed = &rooms[MAX_LIVE_TIMELINES - 1];
    registry
        .open_room_read_timeline(&client, borrowed)
        .await
        .expect("mark-read timeline");
    assert_eq!(registry.entries.len(), MAX_LIVE_TIMELINES);

    registry
        .open(&client, &rooms[MAX_LIVE_TIMELINES])
        .await
        .expect("open room past the cap");
    assert!(
        !registry.entries.contains_key(borrowed),
        "the borrowed mark-read timeline goes first"
    );
    assert!(
        registry.entries.contains_key(&rooms[0]),
        "the oldest room the user opened is kept"
    );
}

#[tokio::test]
async fn mark_read_keeps_a_room_the_user_already_opened_in_use() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let rooms = joined_rooms(&server, &client, MAX_LIVE_TIMELINES + 1).await;
    let mut registry = NativeTimelineRegistry::new(1);

    for room_id in &rooms[..MAX_LIVE_TIMELINES] {
        registry.open(&client, room_id).await.expect("open room");
    }
    // An already-open room is not demoted by Mark as Read; it counts as use.
    registry
        .open_room_read_timeline(&client, &rooms[0])
        .await
        .expect("mark-read timeline");
    registry
        .open(&client, &rooms[MAX_LIVE_TIMELINES])
        .await
        .expect("open room past the cap");
    assert!(registry.entries.contains_key(&rooms[0]));
    assert!(!registry.entries.contains_key(&rooms[1]));
}
