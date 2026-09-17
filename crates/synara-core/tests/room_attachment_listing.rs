//! Empty-term in-room Media/Files listing against the event cache.
//!
//! Does not enable `search-index` and never issues Client-Server `/search`.
#![recursion_limit = "256"]

use matrix_sdk::ruma::{
    events::room::message::{FileMessageEventContent, MessageType, RoomMessageEventContent},
    mxc_uri, room_id, user_id, EventId,
};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
use synara_core::app::search::list_room_attachments;

#[tokio::test]
async fn lists_media_in_range_and_skips_text_and_files() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();

    let room_id = room_id!("!media:localhost");
    let f = EventFactory::new()
        .room(room_id)
        .sender(user_id!("@user_id:localhost"));
    let image = EventId::parse("$img:localhost").expect("event id");
    let text = EventId::parse("$txt:localhost").expect("event id");
    let file = EventId::parse("$file:localhost").expect("event id");
    let in_range = 1_700_000_100_000_u64;
    let too_old = 1_600_000_000_000_u64;

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(
                    f.image(
                        "pic.png".to_owned(),
                        mxc_uri!("mxc://localhost/img").to_owned(),
                    )
                    .event_id(&image)
                    .server_ts(in_range),
                )
                .add_timeline_event(f.text_msg("hello").event_id(&text).server_ts(in_range))
                .add_timeline_event(
                    f.event(RoomMessageEventContent::new(MessageType::File(
                        FileMessageEventContent::plain(
                            "notes.pdf".to_owned(),
                            mxc_uri!("mxc://localhost/file").to_owned(),
                        ),
                    )))
                    .event_id(&file)
                    .server_ts(in_range),
                )
                .add_timeline_event(
                    f.image(
                        "old.png".to_owned(),
                        mxc_uri!("mxc://localhost/old").to_owned(),
                    )
                    .event_id(&EventId::parse("$old:localhost").expect("event id"))
                    .server_ts(too_old),
                ),
        )
        .await;

    let result = list_room_attachments(
        &client,
        room_id.as_str(),
        "media",
        1_700_000_000_000,
        1_700_000_200_000,
    )
    .await
    .expect("listing");

    let ids: Vec<_> = result
        .groups
        .iter()
        .flat_map(|group| group.items.iter().map(|item| item.event_id.as_str()))
        .collect();
    assert_eq!(ids, vec![image.as_str()]);
    assert_eq!(
        result.groups[0].items[0].msg_type.as_deref(),
        Some("m.image")
    );
    assert!(result.highlights.is_empty());
}

#[tokio::test]
async fn lists_files_without_media() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();

    let room_id = room_id!("!files:localhost");
    let f = EventFactory::new()
        .room(room_id)
        .sender(user_id!("@user_id:localhost"));
    let file = EventId::parse("$file:localhost").expect("event id");
    let ts = 1_700_000_100_000_u64;

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(
                    f.event(RoomMessageEventContent::new(MessageType::File(
                        FileMessageEventContent::plain(
                            "notes.pdf".to_owned(),
                            mxc_uri!("mxc://localhost/file").to_owned(),
                        ),
                    )))
                    .event_id(&file)
                    .server_ts(ts),
                )
                .add_timeline_event(
                    f.image(
                        "pic.png".to_owned(),
                        mxc_uri!("mxc://localhost/img").to_owned(),
                    )
                    .event_id(&EventId::parse("$img:localhost").expect("event id"))
                    .server_ts(ts),
                ),
        )
        .await;

    let result = list_room_attachments(
        &client,
        room_id.as_str(),
        "files",
        1_700_000_000_000,
        1_700_000_200_000,
    )
    .await
    .expect("listing");
    assert_eq!(result.groups[0].items.len(), 1);
    assert_eq!(
        result.groups[0].items[0].msg_type.as_deref(),
        Some("m.file")
    );
}

#[test]
fn listing_module_never_mentions_client_server_search() {
    let listing = include_str!("../src/app/search/listing.rs");
    assert!(!listing.contains("search_events"));
    assert!(!listing.contains("/_matrix/client/v3/search"));
    assert!(!listing.contains("search_homeserver"));
}
