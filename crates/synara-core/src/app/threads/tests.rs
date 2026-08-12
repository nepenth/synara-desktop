//! Unit tests for P5.8 thread index.

use super::*;
use crate::dto::ThreadSummary;
use crate::transport::MatrixIpcErrorCategory;

fn summary(
    room: &str,
    root: &str,
    replies: u32,
    latest: Option<&str>,
    ts: Option<u64>,
    participated: bool,
) -> ThreadSummary {
    ThreadSummary {
        room_id: room.into(),
        root_event_id: root.into(),
        reply_count: replies,
        latest_event_id: latest.map(Into::into),
        latest_origin_server_ts: ts,
        participated,
        unread_count: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_threads_markers(), MATRIX_THREADS_MARKER);
}

#[test]
fn upsert_list_and_order() {
    let mut idx = ThreadIndex::new(1);
    idx.upsert(summary(
        "!r:example.org",
        "$old",
        1,
        Some("$a"),
        Some(100),
        false,
    ))
    .unwrap();
    idx.upsert(summary(
        "!r:example.org",
        "$new",
        3,
        Some("$b"),
        Some(500),
        true,
    ))
    .unwrap();
    let list = idx.list_room("!r:example.org");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].root_event_id, "$new");
    assert_eq!(list[1].root_event_id, "$old");
    assert_eq!(
        idx.list_participated("!r:example.org")
            .iter()
            .map(|s| s.root_event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["$new"]
    );
}

#[test]
fn upsert_overwrites() {
    let mut idx = ThreadIndex::new(1);
    idx.upsert(summary(
        "!r:example.org",
        "$root",
        1,
        Some("$e1"),
        Some(1),
        false,
    ))
    .unwrap();
    idx.upsert(summary(
        "!r:example.org",
        "$root",
        5,
        Some("$e5"),
        Some(50),
        true,
    ))
    .unwrap();
    let s = idx.get("!r:example.org", "$root").unwrap();
    assert_eq!(s.reply_count, 5);
    assert_eq!(s.latest_event_id.as_deref(), Some("$e5"));
    assert!(s.participated);
    assert_eq!(idx.thread_count(), 1);
}

#[test]
fn remove_clear_retire() {
    let mut idx = ThreadIndex::new(2);
    idx.upsert(summary("!a:example.org", "$r1", 0, None, None, false))
        .unwrap();
    idx.upsert(summary("!b:example.org", "$r2", 0, None, None, false))
        .unwrap();
    assert!(idx.remove("!a:example.org", "$r1"));
    assert!(!idx.remove("!a:example.org", "$r1"));
    idx.clear_room("!b:example.org");
    assert!(idx.is_empty());
    idx.upsert(summary("!c:example.org", "$r3", 0, None, None, false))
        .unwrap();
    idx.retire_generation(9);
    assert_eq!(idx.session_generation(), 9);
    assert!(idx.is_empty());
}

#[test]
fn invalid_ids_rejected() {
    let mut idx = ThreadIndex::new(1);
    let err = idx
        .upsert(summary("bad", "$r", 0, None, None, false))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.8-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    let err = idx
        .upsert(summary("!r:example.org", "bad", 0, None, None, false))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.8-invalid-root-event-id");
}

#[test]
fn batch_and_cap() {
    let mut idx = ThreadIndex::new(1);
    let n = idx
        .upsert_batch(vec![
            summary("!r:example.org", "$1", 0, None, Some(1), false),
            summary("!r:example.org", "$2", 0, None, Some(2), true),
        ])
        .unwrap();
    assert_eq!(n, 2);
    // Fill up to cap then one more fails.
    let mut idx = ThreadIndex::new(1);
    for i in 0..MAX_THREADS_PER_ROOM {
        idx.upsert(summary(
            "!r:example.org",
            &format!("${i}"),
            0,
            None,
            Some(i as u64),
            false,
        ))
        .unwrap();
    }
    let err = idx
        .upsert(summary(
            "!r:example.org",
            "$overflow",
            0,
            None,
            Some(9999),
            false,
        ))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.8-thread-cap");
    // Overwrite existing still ok at cap.
    idx.upsert(summary(
        "!r:example.org",
        "$0",
        9,
        Some("$z"),
        Some(1),
        true,
    ))
    .unwrap();
    assert_eq!(idx.get("!r:example.org", "$0").unwrap().reply_count, 9);
}
