//! Unit tests for P5.6 relation index.

use super::*;
use crate::dto::{
    RelationRef, REL_TYPE_ANNOTATION, REL_TYPE_REFERENCE, REL_TYPE_REPLACE, REL_TYPE_THREAD,
};
use crate::transport::MatrixIpcErrorCategory;

fn ann(room: &str, target: &str, key: &str, sender: &str) -> RelationRef {
    RelationRef {
        rel_type: REL_TYPE_ANNOTATION.into(),
        event_id: target.into(),
        room_id: Some(room.into()),
        sender: Some(sender.into()),
        key: Some(key.into()),
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_relations_markers(), MATRIX_RELATIONS_MARKER);
}

#[test]
fn annotations_aggregate_and_me() {
    let mut idx = RelationIndex::new(1);
    idx.apply(ann("!r:example.org", "$e1", "👍", "@alice:example.org"))
        .unwrap();
    idx.apply(ann("!r:example.org", "$e1", "👍", "@bob:example.org"))
        .unwrap();
    idx.apply(ann("!r:example.org", "$e1", "🔥", "@alice:example.org"))
        .unwrap();
    let sum = idx.reaction_summaries("!r:example.org", "$e1", Some("@alice:example.org"));
    assert_eq!(sum.len(), 2);
    let by_key: std::collections::BTreeMap<_, _> =
        sum.into_iter().map(|s| (s.key.clone(), s)).collect();
    let fire = by_key.get("🔥").expect("fire");
    assert_eq!(fire.count, 1);
    assert_eq!(fire.me, Some(true));
    let thumb = by_key.get("👍").expect("thumb");
    assert_eq!(thumb.count, 2);
    assert_eq!(thumb.me, Some(true));
}

#[test]
fn remove_annotation() {
    let mut idx = RelationIndex::new(1);
    idx.apply(ann("!r:example.org", "$e1", "x", "@alice:example.org"))
        .unwrap();
    assert!(idx.remove_annotation("!r:example.org", "$e1", "x", "@alice:example.org"));
    assert!(!idx.remove_annotation("!r:example.org", "$e1", "x", "@alice:example.org"));
    assert!(idx
        .reaction_summaries("!r:example.org", "$e1", None)
        .is_empty());
}

#[test]
fn replace_latest_wins() {
    let mut idx = RelationIndex::new(1);
    idx.apply(RelationRef {
        rel_type: REL_TYPE_REPLACE.into(),
        event_id: "$orig".into(),
        room_id: Some("!r:example.org".into()),
        sender: Some("@alice:example.org".into()),
        key: Some("$edit1".into()),
    })
    .unwrap();
    idx.apply(RelationRef {
        rel_type: REL_TYPE_REPLACE.into(),
        event_id: "$orig".into(),
        room_id: Some("!r:example.org".into()),
        sender: Some("@alice:example.org".into()),
        key: Some("$edit2".into()),
    })
    .unwrap();
    let r = idx.latest_replace("!r:example.org", "$orig").unwrap();
    assert_eq!(r.key.as_deref(), Some("$edit2"));
}

#[test]
fn references_and_threads() {
    let mut idx = RelationIndex::new(1);
    idx.apply(RelationRef {
        rel_type: REL_TYPE_REFERENCE.into(),
        event_id: "$target".into(),
        room_id: Some("!r:example.org".into()),
        sender: None,
        key: Some("$src1".into()),
    })
    .unwrap();
    idx.apply(RelationRef {
        rel_type: REL_TYPE_REFERENCE.into(),
        event_id: "$target".into(),
        room_id: Some("!r:example.org".into()),
        sender: None,
        key: Some("$src2".into()),
    })
    .unwrap();
    assert_eq!(idx.reference_count("!r:example.org", "$target"), 2);

    idx.apply(RelationRef {
        rel_type: REL_TYPE_THREAD.into(),
        event_id: "$root".into(),
        room_id: Some("!r:example.org".into()),
        sender: Some("@alice:example.org".into()),
        key: Some("$reply1".into()),
    })
    .unwrap();
    idx.apply(RelationRef {
        rel_type: REL_TYPE_THREAD.into(),
        event_id: "$root".into(),
        room_id: Some("!r:example.org".into()),
        sender: Some("@bob:example.org".into()),
        key: Some("$reply2".into()),
    })
    .unwrap();
    assert_eq!(idx.thread_reply_count("!r:example.org", "$root"), 2);
}

#[test]
fn invalid_ids_rejected() {
    let mut idx = RelationIndex::new(1);
    let err = idx
        .apply(RelationRef {
            rel_type: REL_TYPE_ANNOTATION.into(),
            event_id: "bad".into(),
            room_id: Some("!r:example.org".into()),
            sender: Some("@a:example.org".into()),
            key: Some("k".into()),
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.6-invalid-target-event-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn clear_room_and_retire() {
    let mut idx = RelationIndex::new(3);
    idx.apply(ann("!a:example.org", "$e", "k", "@u:example.org"))
        .unwrap();
    idx.apply(ann("!b:example.org", "$e", "k", "@u:example.org"))
        .unwrap();
    idx.clear_room("!a:example.org");
    assert!(idx
        .reaction_summaries("!a:example.org", "$e", None)
        .is_empty());
    assert_eq!(
        idx.reaction_summaries("!b:example.org", "$e", None).len(),
        1
    );
    idx.retire_generation(4);
    assert_eq!(idx.session_generation(), 4);
    assert!(idx.is_empty());
}

#[test]
fn unsupported_rel_type() {
    let mut idx = RelationIndex::new(1);
    let err = idx
        .apply(RelationRef {
            rel_type: "m.unknown".into(),
            event_id: "$e".into(),
            room_id: Some("!r:example.org".into()),
            sender: None,
            key: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.6-unsupported-rel-type");
}
