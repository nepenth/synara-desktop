//! Desktop `search-index` query path against the 0.19 local index.
//!
//! These tests require `--features search-index`. Workspace `cargo test`
//! without that feature skips them so NSE graphs stay Tantivy-free.
#![recursion_limit = "256"]

use std::time::Duration;

use matrix_sdk::test_utils::mocks::MatrixMockServer;
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
use ruma::{event_id, room_id, user_id, EventId};
use synara_core::app::search::{search_messages, MatrixMessageSearchResult, MESSAGE_SEARCH_LIMIT};

const WAIT: Duration = Duration::from_secs(8);

fn flatten(result: &MatrixMessageSearchResult) -> Vec<(String, String)> {
    result
        .groups
        .iter()
        .flat_map(|group| {
            group
                .items
                .iter()
                .map(|item| (item.room_id.clone(), item.event_id.clone()))
        })
        .collect()
}

async fn wait_search(
    client: &matrix_sdk::Client,
    term: &str,
    rooms: Option<&[String]>,
    min_hits: usize,
) -> MatrixMessageSearchResult {
    tokio::time::timeout(WAIT, async {
        loop {
            let result = search_messages(client, term, None, rooms, None, Some("rank"), true)
                .await
                .expect("indexed search");
            if flatten(&result).len() >= min_hits {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("local index should observe synced timeline events")
}

#[tokio::test]
async fn setting_off_disables_search_without_homeserver_fallback() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let err = search_messages(&client, "secret-term", None, None, None, None, false)
        .await
        .expect_err("off must disable product search");
    assert_eq!(err, "v-search.index-disabled");
    assert!(!err.contains("secret-term"));
    assert!(!err.contains("/search"));
}

#[tokio::test]
async fn global_search_returns_score_interleaved_hits() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();

    let room_id1 = room_id!("!r1:localhost");
    let room_id2 = room_id!("!r2:localhost");
    let f = EventFactory::new().sender(user_id!("@user_id:localhost"));
    let r1_rank1 = event_id!("$r1_rank1:localhost");
    let r2_rank2 = event_id!("$r2_rank2:localhost");
    let r1_rank3 = event_id!("$r1_rank3:localhost");
    let r2_rank4 = event_id!("$r2_rank4:localhost");

    server
        .mock_sync()
        .ok_and_run(&client, |sync_builder| {
            sync_builder
                .add_joined_room(
                    JoinedRoomBuilder::new(room_id1)
                        .add_timeline_event(
                            f.text_msg(
                                "world world world world filler filler filler filler filler filler",
                            )
                            .room(room_id1)
                            .event_id(r1_rank1),
                        )
                        .add_timeline_event(
                            f.text_msg(
                                "world world filler filler filler filler filler filler filler filler",
                            )
                            .room(room_id1)
                            .event_id(r1_rank3),
                        ),
                )
                .add_joined_room(
                    JoinedRoomBuilder::new(room_id2)
                        .add_timeline_event(
                            f.text_msg(
                                "world world world filler filler filler filler filler filler filler",
                            )
                            .room(room_id2)
                            .event_id(r2_rank2),
                        )
                        .add_timeline_event(
                            f.text_msg(
                                "world filler filler filler filler filler filler filler filler filler",
                            )
                            .room(room_id2)
                            .event_id(r2_rank4),
                        ),
                );
        })
        .await;

    let result = wait_search(&client, "world", None, 4).await;
    let hits = flatten(&result);
    assert_eq!(
        hits,
        vec![
            (room_id1.to_string(), r1_rank1.to_string()),
            (room_id2.to_string(), r2_rank2.to_string()),
            (room_id1.to_string(), r1_rank3.to_string()),
            (room_id2.to_string(), r2_rank4.to_string()),
        ]
    );
    assert!(result.highlights.iter().any(|value| value == "world"));
    // Consecutive grouping keeps interleave: adjacent hits share a room only
    // when the score order itself is consecutive in that room.
    assert!(result.groups.len() >= 2);
    assert_ne!(result.groups[0].room_id, result.groups[1].room_id);
}

#[tokio::test]
async fn room_scoped_search_does_not_return_other_rooms() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();

    let room_id1 = room_id!("!scoped-a:localhost");
    let room_id2 = room_id!("!scoped-b:localhost");
    let f = EventFactory::new().sender(user_id!("@user_id:localhost"));
    let in_room = event_id!("$in-room:localhost");
    let other = event_id!("$other-room:localhost");

    server
        .mock_sync()
        .ok_and_run(&client, |sync_builder| {
            sync_builder
                .add_joined_room(JoinedRoomBuilder::new(room_id1).add_timeline_event(
                    f.text_msg("alpha needle").room(room_id1).event_id(in_room),
                ))
                .add_joined_room(
                    JoinedRoomBuilder::new(room_id2).add_timeline_event(
                        f.text_msg("beta needle").room(room_id2).event_id(other),
                    ),
                );
        })
        .await;

    let rooms = [room_id1.to_string()];
    let result = wait_search(&client, "needle", Some(&rooms), 1).await;
    let hits = flatten(&result);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0], (room_id1.to_string(), in_room.to_string()));
    assert!(hits.iter().all(|(room, _)| room == room_id1.as_str()));
}

#[tokio::test]
async fn pagination_uses_decimal_offset_without_duplicates() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();

    let room_id = room_id!("!page:localhost");
    let f = EventFactory::new()
        .room(room_id)
        .sender(user_id!("@user_id:localhost"));
    let mut builder = JoinedRoomBuilder::new(room_id);
    let total = usize::from(MESSAGE_SEARCH_LIMIT) + 5;
    for index in 0..total {
        let event_id = EventId::parse(format!("$page-{index}:localhost")).expect("event id");
        builder = builder.add_timeline_event(
            f.text_msg("paginated needle shared filler filler filler")
                .event_id(&event_id),
        );
    }
    server.sync_room(&client, builder).await;

    let first = wait_search(
        &client,
        "paginated",
        None,
        usize::from(MESSAGE_SEARCH_LIMIT),
    )
    .await;
    let first_hits = flatten(&first);
    assert_eq!(first_hits.len(), usize::from(MESSAGE_SEARCH_LIMIT));
    let token = first.next_token.expect("second page token");
    assert!(token.bytes().all(|byte| byte.is_ascii_digit()));
    assert!(!token.contains("paginated"));

    let second = search_messages(
        &client,
        "paginated",
        Some(token.as_str()),
        None,
        None,
        Some("rank"),
        true,
    )
    .await
    .expect("second page");
    let second_hits = flatten(&second);
    assert!(!second_hits.is_empty());
    let first_ids: Vec<&str> = first_hits.iter().map(|(_, id)| id.as_str()).collect();
    for (_, event_id) in &second_hits {
        assert!(
            !first_ids.contains(&event_id.as_str()),
            "pages must not overlap: {event_id}"
        );
    }
}

#[tokio::test]
async fn decrypted_body_is_a_hit_and_ciphertext_payload_is_not() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();

    let room_id = room_id!("!plain:localhost");
    let f = EventFactory::new()
        .room(room_id)
        .sender(user_id!("@user_id:localhost"));
    let decrypted = event_id!("$decrypted-body:localhost");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(f.text_msg("unique-decrypted-body").event_id(decrypted)),
        )
        .await;

    let hit = wait_search(&client, "unique-decrypted-body", None, 1).await;
    assert_eq!(flatten(&hit)[0].1, decrypted.to_string());

    let ciphertext = search_messages(
        &client,
        "cipher-not-indexed",
        None,
        None,
        None,
        Some("rank"),
        true,
    )
    .await
    .expect("missing ciphertext term");
    assert!(flatten(&ciphertext).is_empty());
}
