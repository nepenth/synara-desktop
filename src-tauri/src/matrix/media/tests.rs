//! Unit tests for P6.4 media upload queue.

use super::*;
use crate::matrix::dto::UploadState;
use crate::matrix::ipc::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_media_markers(), MATRIX_MEDIA_MARKER);
}

#[test]
fn enqueue_begin_progress_complete() {
    let mut q = UploadQueue::new(2);
    assert_eq!(q.session_generation(), 2);
    let id = q
        .enqueue(
            "photo.jpg",
            Some("!r:example.org".into()),
            Some("image/jpeg".into()),
            Some(1024),
        )
        .unwrap()
        .upload_id
        .clone();
    assert_eq!(q.get(&id).unwrap().state, UploadState::Queued);
    assert_eq!(q.active_count(), 1);

    q.begin(&id).unwrap();
    assert_eq!(q.get(&id).unwrap().state, UploadState::Uploading);
    q.set_progress(&id, 0.5).unwrap();
    assert_eq!(q.get(&id).unwrap().progress01, Some(0.5));
    q.complete(&id, "mxc://example.org/abc").unwrap();
    let done = q.get(&id).unwrap();
    assert_eq!(done.state, UploadState::Completed);
    assert_eq!(done.progress01, Some(1.0));
    assert_eq!(
        done.media_handle_id.as_deref(),
        Some("mxc://example.org/abc")
    );
    assert_eq!(q.active_count(), 0);
}

#[test]
fn fail_and_cancel() {
    let mut q = UploadQueue::new(1);
    let id = q
        .enqueue("a.bin", None, None, Some(10))
        .unwrap()
        .upload_id
        .clone();
    q.begin(&id).unwrap();
    q.fail(&id, "p6.4-network-failed").unwrap();
    assert_eq!(q.get(&id).unwrap().state, UploadState::Failed);
    assert_eq!(q.failure_diagnostic(&id), Some("p6.4-network-failed"));
    assert!(!q.failure_diagnostic(&id).unwrap().contains("access_token"));

    let id2 = q
        .enqueue("b.bin", None, None, Some(10))
        .unwrap()
        .upload_id
        .clone();
    q.cancel(&id2).unwrap();
    assert_eq!(q.get(&id2).unwrap().state, UploadState::Cancelled);
}

#[test]
fn validation_and_cap() {
    let mut q = UploadQueue::new(1);
    let err = q.enqueue("", None, None, None).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.4-empty-file-name");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    let err = q.enqueue("x", Some("bad".into()), None, None).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.4-invalid-room-id");

    let err = q
        .enqueue("x", None, None, Some(200 * 1024 * 1024))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.4-file-too-large");

    for i in 0..MAX_ACTIVE_UPLOADS {
        q.enqueue(format!("f{i}.bin"), None, None, Some(1)).unwrap();
    }
    let err = q.enqueue("overflow.bin", None, None, Some(1)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.4-active-upload-cap");
}

#[test]
fn prune_and_retire() {
    let mut q = UploadQueue::new(1);
    let a = q
        .enqueue("a.bin", None, None, Some(1))
        .unwrap()
        .upload_id
        .clone();
    let b = q
        .enqueue("b.bin", None, None, Some(1))
        .unwrap()
        .upload_id
        .clone();
    q.begin(&a).unwrap();
    q.complete(&a, "h1").unwrap();
    assert_eq!(q.prune_terminal(), 1);
    assert!(q.get(&a).is_none());
    assert!(q.get(&b).is_some());

    q.retire_generation(9);
    assert_eq!(q.session_generation(), 9);
    assert_eq!(q.get(&b).unwrap().state, UploadState::Cancelled);
}

#[test]
fn progress_clamped() {
    let mut q = UploadQueue::new(1);
    let id = q
        .enqueue("a.bin", None, None, Some(1))
        .unwrap()
        .upload_id
        .clone();
    q.begin(&id).unwrap();
    q.set_progress(&id, 2.5).unwrap();
    assert_eq!(q.get(&id).unwrap().progress01, Some(1.0));
    q.set_progress(&id, -1.0).unwrap();
    assert_eq!(q.get(&id).unwrap().progress01, Some(0.0));
}
