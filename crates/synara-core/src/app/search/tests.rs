//! Unit tests for P6.8 search session.

use super::*;
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
