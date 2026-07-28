//! Unit tests for P6.7 account-data index.

use super::*;
use std::collections::BTreeMap;

#[test]
fn marker_stable() {
    assert_eq!(matrix_account_data_markers(), MATRIX_ACCOUNT_DATA_MARKER);
}

#[test]
fn fully_read_helper() {
    let mut idx = AccountDataIndex::new(1);
    idx.set_fully_read("!r:example.org", "$e1:example.org")
        .unwrap();
    assert_eq!(
        idx.fully_read_event_id("!r:example.org"),
        Some("$e1:example.org")
    );
    idx.set_fully_read("!r:example.org", "$e2:example.org")
        .unwrap();
    assert_eq!(
        idx.fully_read_event_id("!r:example.org"),
        Some("$e2:example.org")
    );
}

#[test]
fn global_and_room_upsert() {
    let mut idx = AccountDataIndex::new(2);
    let mut fields = BTreeMap::new();
    fields.insert("enabled".into(), "true".into());
    idx.upsert(AccountDataEntry {
        event_type: TYPE_PUSH_RULES.into(),
        room_id: None,
        fields,
    })
    .unwrap();
    assert!(idx.get_global(TYPE_PUSH_RULES).is_some());
    assert_eq!(idx.list_global_types(), vec![TYPE_PUSH_RULES]);

    let mut tags = BTreeMap::new();
    tags.insert("m.favourite".into(), "1".into());
    idx.upsert(AccountDataEntry {
        event_type: TYPE_TAG.into(),
        room_id: Some("!r:example.org".into()),
        fields: tags,
    })
    .unwrap();
    assert_eq!(
        idx.get_room("!r:example.org", TYPE_TAG)
            .unwrap()
            .get("m.favourite"),
        Some("1")
    );
}

#[test]
fn forbids_secret_keys_and_values() {
    let mut idx = AccountDataIndex::new(1);
    let mut fields = BTreeMap::new();
    fields.insert("access_token".into(), "x".into());
    let err = idx
        .upsert(AccountDataEntry {
            event_type: "m.custom".into(),
            room_id: None,
            fields,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.7-forbidden-field-key");

    let mut fields = BTreeMap::new();
    fields.insert("note".into(), "access_token=abc".into());
    let err = idx
        .upsert(AccountDataEntry {
            event_type: "m.custom".into(),
            room_id: None,
            fields,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.7-forbidden-field-value");
}

#[test]
fn remove_and_retire() {
    let mut idx = AccountDataIndex::new(1);
    idx.set_fully_read("!r:example.org", "$e:example.org")
        .unwrap();
    assert!(idx.remove_room("!r:example.org", TYPE_FULLY_READ));
    assert!(idx.fully_read_event_id("!r:example.org").is_none());
    idx.upsert(AccountDataEntry {
        event_type: TYPE_DIRECT.into(),
        room_id: None,
        fields: BTreeMap::new(),
    })
    .unwrap();
    assert!(idx.remove_global(TYPE_DIRECT));
    idx.retire_generation(9);
    assert!(idx.is_empty());
    assert_eq!(idx.session_generation(), 9);
}

#[test]
fn validation() {
    let mut idx = AccountDataIndex::new(1);
    let err = idx.set_fully_read("bad", "$e:example.org").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.7-invalid-room-id");
    let err = idx
        .set_fully_read("!r:example.org", "not-event")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.7-invalid-event-id");
}
