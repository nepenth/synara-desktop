//! Unit tests for P7.5 metadata-only media export intents.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

fn enqueue(queue: &mut ExportQueue, kind: ExportKind) -> ExportJobId {
    queue
        .enqueue(
            kind,
            "opaque-media-handle",
            Some("!room:example.org".into()),
        )
        .unwrap()
        .id
        .clone()
}

#[test]
fn marker_is_stable() {
    assert_eq!(matrix_media_export_markers(), MATRIX_MEDIA_EXPORT_MARKER);
}

#[test]
fn all_export_kinds_follow_success_lifecycle() {
    let mut queue = ExportQueue::new(7);

    for kind in [
        ExportKind::Save,
        ExportKind::Share,
        ExportKind::Open,
        ExportKind::Drag,
    ] {
        let id = enqueue(&mut queue, kind);
        let pending = queue.get(&id).unwrap();
        assert_eq!(pending.kind, kind);
        assert_eq!(pending.state, ExportState::Pending);

        assert_eq!(queue.start(&id).unwrap().state, ExportState::Running);
        assert_eq!(queue.complete(&id).unwrap().state, ExportState::Succeeded);
    }

    assert_eq!(queue.session_generation(), 7);
    assert_eq!(queue.len(), 4);
}

#[test]
fn fail_cancel_and_invalid_transitions_are_enforced() {
    let mut queue = ExportQueue::new(1);
    let failed = enqueue(&mut queue, ExportKind::Share);
    let cancelled = enqueue(&mut queue, ExportKind::Open);

    assert_eq!(queue.fail(&failed).unwrap().state, ExportState::Failed);
    queue.start(&cancelled).unwrap();
    assert_eq!(
        queue.cancel(&cancelled).unwrap().state,
        ExportState::Cancelled
    );

    let error = queue.start(&failed).unwrap_err();
    assert_eq!(error.diagnostic_id(), "p7.5-start-not-pending");
    assert_eq!(error.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        queue.complete("missing").unwrap_err().diagnostic_id(),
        "p7.5-export-not-found"
    );
}

#[test]
fn validation_never_echoes_metadata_in_errors() {
    let mut queue = ExportQueue::new(1);
    let secret_shaped_handle = "access_token=do-not-echo";

    let error = queue
        .enqueue(ExportKind::Save, "", None)
        .expect_err("empty handle must fail");
    assert_eq!(error.diagnostic_id(), "p7.5-empty-media-handle");
    assert!(!format!("{error:?} {error}").contains(secret_shaped_handle));

    let error = queue
        .enqueue(
            ExportKind::Drag,
            secret_shaped_handle,
            Some("not-a-room".into()),
        )
        .expect_err("invalid room must fail");
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains(secret_shaped_handle));
    assert!(!rendered.contains("not-a-room"));
}

#[test]
fn debug_redacts_handle_and_room_id() {
    let mut queue = ExportQueue::new(1);
    let id = queue
        .enqueue(
            ExportKind::Save,
            "opaque-private-handle",
            Some("!private-room:example.org".into()),
        )
        .unwrap()
        .id
        .clone();

    let rendered = format!("{:?}", queue.get(&id).unwrap());
    assert!(!rendered.contains("opaque-private-handle"));
    assert!(!rendered.contains("!private-room:example.org"));
    assert!(rendered.contains("<opaque>"));
}

#[test]
fn pruning_and_generation_retirement_only_change_state() {
    let mut queue = ExportQueue::new(2);
    let succeeded = enqueue(&mut queue, ExportKind::Save);
    let running = enqueue(&mut queue, ExportKind::Share);
    let pending = enqueue(&mut queue, ExportKind::Drag);

    queue.start(&succeeded).unwrap();
    queue.complete(&succeeded).unwrap();
    queue.start(&running).unwrap();

    assert_eq!(queue.prune_terminal(), 1);
    assert!(queue.get(&succeeded).is_none());

    queue.retire_generation(3);
    assert_eq!(queue.session_generation(), 3);
    assert_eq!(queue.get(&running).unwrap().state, ExportState::Cancelled);
    assert_eq!(queue.get(&pending).unwrap().state, ExportState::Cancelled);
    assert_eq!(queue.prune_terminal(), 2);
    assert!(queue.is_empty());
}

#[test]
fn listing_preserves_enqueue_order() {
    let mut queue = ExportQueue::new(1);
    let first = enqueue(&mut queue, ExportKind::Open);
    let second = enqueue(&mut queue, ExportKind::Drag);

    let ids: Vec<&str> = queue.list().iter().map(|job| job.id.as_str()).collect();
    assert_eq!(ids, [first, second]);
}
