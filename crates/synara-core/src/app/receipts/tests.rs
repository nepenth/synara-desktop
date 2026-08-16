//! Unit tests for P6.2 receipt index.

use super::*;
use crate::dto::{Receipt, ReceiptType};
use crate::transport::MatrixIpcErrorCategory;

fn receipt(room: &str, event: &str, user: &str, kind: ReceiptType, ts: Option<u64>) -> Receipt {
    Receipt {
        room_id: room.into(),
        event_id: event.into(),
        user_id: user.into(),
        receipt_type: kind,
        ts,
        thread_id: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_receipts_markers(), MATRIX_RECEIPTS_MARKER);
}

#[test]
fn apply_and_latest_read() {
    let mut idx = ReceiptIndex::new(2);
    idx.apply(receipt(
        "!r:example.org",
        "$e1",
        "@alice:example.org",
        ReceiptType::Read,
        Some(100),
    ))
    .unwrap();
    idx.apply(receipt(
        "!r:example.org",
        "$e2",
        "@alice:example.org",
        ReceiptType::Read,
        Some(200),
    ))
    .unwrap();
    let latest = idx
        .latest_read("!r:example.org", "@alice:example.org")
        .unwrap();
    assert_eq!(latest.event_id, "$e2");
    assert_eq!(latest.ts, Some(200));
    assert_eq!(idx.list_room("!r:example.org").len(), 1);
}

#[test]
fn older_ts_does_not_overwrite() {
    let mut idx = ReceiptIndex::new(1);
    idx.apply(receipt(
        "!r:example.org",
        "$new",
        "@bob:example.org",
        ReceiptType::Read,
        Some(500),
    ))
    .unwrap();
    idx.apply(receipt(
        "!r:example.org",
        "$old",
        "@bob:example.org",
        ReceiptType::Read,
        Some(100),
    ))
    .unwrap();
    let latest = idx
        .latest_read("!r:example.org", "@bob:example.org")
        .unwrap();
    assert_eq!(latest.event_id, "$new");
}

#[test]
fn types_and_users_independent() {
    let mut idx = ReceiptIndex::new(1);
    idx.apply(receipt(
        "!r:example.org",
        "$a",
        "@alice:example.org",
        ReceiptType::Read,
        Some(1),
    ))
    .unwrap();
    idx.apply(receipt(
        "!r:example.org",
        "$b",
        "@alice:example.org",
        ReceiptType::ReadPrivate,
        Some(2),
    ))
    .unwrap();
    idx.apply(receipt(
        "!r:example.org",
        "$c",
        "@carol:example.org",
        ReceiptType::FullyRead,
        Some(3),
    ))
    .unwrap();
    assert_eq!(idx.list_room("!r:example.org").len(), 3);
}

#[test]
fn invalid_ids_rejected() {
    let mut idx = ReceiptIndex::new(1);
    let err = idx
        .apply(receipt(
            "bad",
            "$e",
            "@u:example.org",
            ReceiptType::Read,
            None,
        ))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.2-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    let err = idx
        .apply(receipt(
            "!r:example.org",
            "not-event",
            "@u:example.org",
            ReceiptType::Read,
            None,
        ))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.2-invalid-event-id");
}

#[test]
fn retire_generation_wipes() {
    let mut idx = ReceiptIndex::new(1);
    idx.apply(receipt(
        "!r:example.org",
        "$e",
        "@a:example.org",
        ReceiptType::Read,
        Some(1),
    ))
    .unwrap();
    idx.retire_generation(9);
    assert_eq!(idx.session_generation(), 9);
    assert!(idx.is_empty());
}

#[test]
fn clear_room_and_batch() {
    let mut idx = ReceiptIndex::new(1);
    let n = idx
        .apply_batch(vec![
            receipt(
                "!a:example.org",
                "$1",
                "@a:example.org",
                ReceiptType::Read,
                Some(1),
            ),
            receipt(
                "!b:example.org",
                "$2",
                "@b:example.org",
                ReceiptType::Read,
                Some(2),
            ),
        ])
        .unwrap();
    assert_eq!(n, 2);
    assert_eq!(idx.room_count(), 2);
    idx.clear_room("!a:example.org");
    assert_eq!(idx.room_count(), 1);
    idx.clear();
    assert!(idx.is_empty());
}
