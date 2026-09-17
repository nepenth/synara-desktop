//! Unit tests for P6.8 search session.

use super::*;
use super::{listing, live};
use crate::dto::{SearchResult, SearchResultItem};
use crate::transport::MatrixIpcErrorCategory;

fn item(event: &str, room: &str) -> SearchResultItem {
    SearchResultItem {
        event_id: event.into(),
        room_id: room.into(),
        origin_server_ts: Some(1),
        sender: Some("@a:example.org".into()),
        snippet: Some("hi".into()),
    }
}

fn page(query: &str, events: &[&str]) -> SearchResult {
    SearchResult {
        query: query.into(),
        room_id: None,
        results: events.iter().map(|e| item(e, "!r:example.org")).collect(),
        next_batch: Some("b1".into()),
        total_count: Some(events.len() as u32),
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_search_markers(), MATRIX_SEARCH_MARKER);
}

#[test]
fn begin_apply_snapshot() {
    let mut s = SearchSession::new(1);
    let rid = s.begin("hello", None).unwrap();
    assert_eq!(s.state(), SearchState::InFlight);
    assert!(s
        .apply_page(rid, page("hello", &["$e1", "$e2"]), false)
        .unwrap());
    assert_eq!(s.state(), SearchState::Ready);
    assert_eq!(s.items().len(), 2);
    let snap = s.to_result();
    assert_eq!(snap.query, "hello");
    assert_eq!(snap.results.len(), 2);
    assert_eq!(snap.next_batch.as_deref(), Some("b1"));
}

#[test]
fn stale_request_ignored() {
    let mut s = SearchSession::new(1);
    let r1 = s.begin("a", None).unwrap();
    let r2 = s.begin("b", None).unwrap();
    assert_ne!(r1, r2);
    assert!(!s.apply_page(r1, page("a", &["$old"]), false).unwrap());
    assert!(s.apply_page(r2, page("b", &["$new"]), false).unwrap());
    assert_eq!(s.items().len(), 1);
    assert_eq!(s.items()[0].event_id, "$new");
}

#[test]
fn cancel_blocks_apply() {
    let mut s = SearchSession::new(1);
    let rid = s.begin("q", None).unwrap();
    s.cancel();
    assert_eq!(s.state(), SearchState::Cancelled);
    let err = s.apply_page(rid, page("q", &["$e"]), false).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.8-apply-after-cancel");
    assert_eq!(err.category(), MatrixIpcErrorCategory::Cancellation);
}

#[test]
fn append_dedup_and_cap() {
    let mut s = SearchSession::new(1);
    let rid = s.begin("q", None).unwrap();
    s.apply_page(rid, page("q", &["$e1"]), false).unwrap();
    s.apply_page(rid, page("q", &["$e1", "$e2"]), true).unwrap();
    assert_eq!(s.items().len(), 2);
}

#[test]
fn validation() {
    let mut s = SearchSession::new(1);
    assert!(s.begin("", None).is_err());
    assert!(s.begin("q", Some("bad".into())).is_err());
    let rid = s.begin("q", Some("!r:example.org".into())).unwrap();
    let bad = SearchResult {
        query: "other".into(),
        room_id: None,
        results: vec![item("$e", "!r:example.org")],
        next_batch: None,
        total_count: None,
    };
    assert!(s.apply_page(rid, bad, false).is_err());
}

#[test]
fn fail_and_retire() {
    let mut s = SearchSession::new(2);
    let rid = s.begin("q", None).unwrap();
    assert!(s.fail(rid, "p6.8-homeserver").unwrap());
    assert_eq!(s.state(), SearchState::Failed);
    assert_eq!(s.failure_diagnostic_id(), Some("p6.8-homeserver"));
    s.retire_generation(3);
    assert_eq!(s.session_generation(), 3);
    assert_eq!(s.state(), SearchState::Idle);
}

#[test]
fn message_search_validates_without_echo() {
    assert_eq!(parse_message_search_term("   ").unwrap(), None);
    let too_long = "t".repeat(MAX_MESSAGE_SEARCH_TERM_CHARS + 1);
    assert_eq!(
        parse_message_search_term(&too_long).unwrap_err(),
        "v-search.term-too-long"
    );
    assert_eq!(
        parse_message_search_term("access_token=syt_secret").unwrap_err(),
        "v-search.invalid-term"
    );
    assert_eq!(
        parse_message_search_rooms(Some(&["not-a-room".into()])).unwrap_err(),
        "v-search.invalid-room"
    );
    assert_eq!(
        parse_message_search_senders(Some(&["not-a-user".into()])).unwrap_err(),
        "v-search.invalid-sender"
    );
    assert_eq!(
        parse_message_search_order(Some("oldest")).unwrap_err(),
        "v-search.invalid-order"
    );
    assert_eq!(
        parse_message_search_next_token(Some("syt_secret")).unwrap_err(),
        "v-search.invalid-token"
    );
    assert_eq!(
        parse_message_search_term("hello").unwrap().as_deref(),
        Some("hello")
    );
    parse_message_search_order(None).expect("omitted order is rank");
    parse_message_search_order(Some("rank")).expect("rank is accepted");
    parse_message_search_order(Some("recent")).expect("recent is accepted as rank");
    let secret_token = parse_message_search_next_token(Some("syt_secret")).unwrap_err();
    assert_eq!(secret_token, "v-search.invalid-token");
    assert!(!secret_token.contains("syt_"));
}

#[cfg(feature = "search-index")]
#[test]
fn message_search_offset_tokens_are_decimal_without_echo() {
    assert_eq!(parse_message_search_offset(None).unwrap(), 0);
    assert_eq!(parse_message_search_offset(Some("20")).unwrap(), 20);
    assert_eq!(
        parse_message_search_next_token(Some("20"))
            .unwrap()
            .as_deref(),
        Some("20")
    );
    let invalid = parse_message_search_next_token(Some("nb-secret")).unwrap_err();
    assert_eq!(invalid, "v-search.invalid-token");
    assert!(!invalid.contains("nb-secret"));
}

#[test]
fn desktop_index_source_never_sends_search_events() {
    let index = include_str!("index.rs");
    assert!(index.contains("client.search_messages"));
    assert!(!index.contains("search::search_events"));
    assert!(!index.contains("client.send("));
    assert!(!index.contains("/_matrix/client"));
    assert!(!index.contains("UnencryptedDirectory"));
}

#[test]
fn message_search_dto_is_ids_and_snippet_only() {
    let result = MatrixMessageSearchResult {
        next_token: Some("nb1".into()),
        highlights: vec!["hello".into()],
        groups: vec![MatrixMessageSearchGroup {
            room_id: "!r:example.org".into(),
            items: vec![MatrixMessageSearchItem {
                rank: 0.5,
                event_id: "$e".into(),
                sender: "@a:example.org".into(),
                origin_server_ts: 1,
                body: "hello".into(),
                room_id: "!r:example.org".into(),
                msg_type: Some("m.text".into()),
            }],
        }],
    };
    let json = serde_json::to_value(&result).expect("message search serializes");
    assert_eq!(json["nextToken"], "nb1");
    assert_eq!(json["highlights"][0], "hello");
    assert_eq!(json["groups"][0]["roomId"], "!r:example.org");
    assert_eq!(json["groups"][0]["items"][0]["eventId"], "$e");
    assert_eq!(json["groups"][0]["items"][0]["body"], "hello");
    assert_eq!(json["groups"][0]["items"][0]["msgType"], "m.text");
    assert!(json.get("term").is_none());
    assert!(json.get("search_categories").is_none());
    assert!(json["groups"][0]["items"][0].get("result").is_none());
    assert!(json["groups"][0]["items"][0].get("event").is_none());
    assert!(json["groups"][0]["items"][0].get("content").is_none());
}

#[test]
fn map_hit_event_preserves_known_msg_type_and_skips_garbage() {
    let image = live::map_hit_event_from_json(
        1.0,
        None,
        serde_json::json!({
            "event_id": "$img",
            "sender": "@a:example.org",
            "origin_server_ts": 1_700_000_000_000_u64,
            "room_id": "!r:example.org",
            "content": { "msgtype": "m.image", "body": "pic.png" }
        }),
    )
    .expect("image hit");
    assert_eq!(image.msg_type.as_deref(), Some("m.image"));
    assert_eq!(image.body, "pic.png");
    assert_eq!(image.origin_server_ts, 1_700_000_000_000);

    let file = live::map_hit_event_from_json(
        1.0,
        None,
        serde_json::json!({
            "event_id": "$file",
            "sender": "@a:example.org",
            "origin_server_ts": 1_700_000_000_001_u64,
            "room_id": "!r:example.org",
            "content": { "msgtype": "m.file", "filename": "notes.pdf" }
        }),
    )
    .expect("file hit");
    assert_eq!(file.msg_type.as_deref(), Some("m.file"));

    let garbage = live::map_hit_event_from_json(
        1.0,
        None,
        serde_json::json!({
            "event_id": "$bad",
            "sender": "@a:example.org",
            "origin_server_ts": 2,
            "room_id": "!r:example.org",
            "content": { "msgtype": "http://evil.example/m.image", "body": "no" }
        }),
    )
    .expect("garbage msgtype still maps the hit");
    assert_eq!(garbage.msg_type, None);

    assert_eq!(live::accept_msg_type("m.video").as_deref(), Some("m.video"));
    assert_eq!(
        live::accept_msg_type("m.poll.start").as_deref(),
        Some("m.poll.start")
    );
    assert!(live::accept_msg_type("not-a-type").is_none());
    assert!(live::accept_msg_type("m.").is_none());
    assert!(live::accept_msg_type(&"m.x".repeat(40)).is_none());
}

#[test]
fn attachment_listing_kind_matches_media_and_files() {
    assert_eq!(
        parse_attachment_listing_kind("media").unwrap(),
        AttachmentListingKind::Media
    );
    assert_eq!(
        parse_attachment_listing_kind("files").unwrap(),
        AttachmentListingKind::Files
    );
    assert_eq!(
        parse_attachment_listing_kind("audio").unwrap_err(),
        "v-search.invalid-listing"
    );
    assert!(listing::msg_type_matches_listing(
        Some("m.image"),
        AttachmentListingKind::Media
    ));
    assert!(listing::msg_type_matches_listing(
        Some("m.video"),
        AttachmentListingKind::Media
    ));
    assert!(!listing::msg_type_matches_listing(
        Some("m.file"),
        AttachmentListingKind::Media
    ));
    assert!(listing::msg_type_matches_listing(
        Some("m.file"),
        AttachmentListingKind::Files
    ));
    assert!(!listing::msg_type_matches_listing(
        Some("m.audio"),
        AttachmentListingKind::Files
    ));
}

#[test]
fn desktop_listing_source_never_sends_search_events() {
    let listing = include_str!("listing.rs");
    assert!(listing.contains("MessagesOptions::backward"));
    assert!(listing.contains("room.messages"));
    assert!(!listing.contains("search::search_events"));
    assert!(!listing.contains("/_matrix/client/v3/search"));
    assert!(!listing.contains("search_homeserver"));
}
