//! Unit tests for P5.1 timeline registry.

use super::*;
use crate::matrix::ipc::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_timeline_markers(), MATRIX_TIMELINE_MARKER);
}

#[test]
fn open_mark_live_close_dispose() {
    let mut reg = TimelineRegistry::new(3);
    let key = TimelineKey::main("!room:example.org").unwrap();
    let e = reg.open(key.clone()).unwrap();
    assert_eq!(e.lifecycle, TimelineLifecycle::Opening);
    assert_eq!(e.session_generation, 3);
    assert_eq!(reg.active_count(), 1);

    reg.mark_live(&key).unwrap();
    assert_eq!(reg.get(&key).unwrap().lifecycle, TimelineLifecycle::Live);

    reg.close(&key).unwrap();
    assert_eq!(reg.get(&key).unwrap().lifecycle, TimelineLifecycle::Closed);
    assert_eq!(reg.active_count(), 0);

    // Reopen after close.
    reg.open(key.clone()).unwrap();
    reg.mark_live(&key).unwrap();

    reg.dispose(&key).unwrap();
    assert!(reg.get(&key).is_none());
    assert!(reg.is_empty());
}

#[test]
fn already_open_rejected() {
    let mut reg = TimelineRegistry::new(1);
    let key = TimelineKey::main("!r:example.org").unwrap();
    reg.open(key.clone()).unwrap();
    let err = reg.open(key).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.1-timeline-already-open");
}

#[test]
fn thread_key_distinct_from_main() {
    let mut reg = TimelineRegistry::new(1);
    let main = TimelineKey::main("!r:example.org").unwrap();
    let thr = TimelineKey::thread("!r:example.org", "$root").unwrap();
    reg.open(main.clone()).unwrap();
    reg.open(thr.clone()).unwrap();
    assert_eq!(reg.len(), 2);
    reg.mark_live(&main).unwrap();
    reg.mark_live(&thr).unwrap();
    assert_eq!(reg.active_count(), 2);
}

#[test]
fn retire_generation_closes_active() {
    let mut reg = TimelineRegistry::new(1);
    let key = TimelineKey::main("!r:example.org").unwrap();
    reg.open(key.clone()).unwrap();
    reg.mark_live(&key).unwrap();
    reg.retire_generation(2);
    assert_eq!(reg.session_generation(), 2);
    assert_eq!(reg.get(&key).unwrap().lifecycle, TimelineLifecycle::Closed);
    assert_eq!(reg.active_count(), 0);
    // Can open again under new generation.
    reg.open(key.clone()).unwrap();
    assert_eq!(reg.get(&key).unwrap().session_generation, 2);
}

#[test]
fn mark_failed_sets_diagnostic() {
    let mut reg = TimelineRegistry::new(1);
    let key = TimelineKey::main("!r:example.org").unwrap();
    reg.open(key.clone()).unwrap();
    reg.mark_failed(&key, "p5.1-attach-failed").unwrap();
    let e = reg.get(&key).unwrap();
    assert_eq!(e.lifecycle, TimelineLifecycle::Failed);
    assert_eq!(e.failure_diagnostic_id, Some("p5.1-attach-failed"));
}

#[test]
fn invalid_room_id_rejected() {
    let err = TimelineKey::main("not-a-room").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.1-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert!(!err.to_string().contains("access_token"));
}

#[test]
fn clear_wipes_registry() {
    let mut reg = TimelineRegistry::new(1);
    reg.open(TimelineKey::main("!a:example.org").unwrap())
        .unwrap();
    reg.clear();
    assert!(reg.is_empty());
}
