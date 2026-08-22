//! Unit tests for P6.4 media upload + P7.2 media download queues.

use super::*;
use crate::dto::UploadState;
use crate::transport::MatrixIpcErrorCategory;

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

#[test]
fn download_lifecycle() {
    let mut q = DownloadQueue::new(3);
    let id = q
        .enqueue(
            "mxc://example.org/abc",
            DownloadKind::Original,
            Some("!r:example.org".into()),
        )
        .unwrap()
        .download_id
        .clone();
    assert_eq!(q.get(&id).unwrap().state, DownloadState::Queued);
    q.begin(&id).unwrap();
    q.set_progress(&id, 0.4).unwrap();
    q.complete(&id, "local-cache:abc").unwrap();
    let done = q.get(&id).unwrap();
    assert_eq!(done.state, DownloadState::Ready);
    assert_eq!(done.local_handle_id.as_deref(), Some("local-cache:abc"));
    assert_eq!(done.progress01, Some(1.0));
}

#[test]
fn download_fail_retry_cancel_prune() {
    let mut q = DownloadQueue::new(1);
    let id = q
        .enqueue("mxc://example.org/x", DownloadKind::Thumbnail, None)
        .unwrap()
        .download_id
        .clone();
    q.begin(&id).unwrap();
    q.fail(&id, "p7.2-network-failed").unwrap();
    assert_eq!(
        q.get(&id).unwrap().failure_diagnostic_id,
        Some("p7.2-network-failed")
    );
    q.retry(&id).unwrap();
    assert_eq!(q.get(&id).unwrap().state, DownloadState::Queued);
    q.cancel(&id).unwrap();
    assert_eq!(q.prune_terminal(), 1);
    assert!(q.is_empty());
}

#[test]
fn download_forbids_data_and_tokens() {
    let mut q = DownloadQueue::new(1);
    let err = q
        .enqueue("data:image/png;base64,AAA", DownloadKind::Avatar, None)
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.2-forbidden-media-scheme");
    let err = q
        .enqueue(
            "mxc://example.org/x?access_token=secret",
            DownloadKind::Original,
            None,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.2-forbidden-media-id");
    let id = q
        .enqueue("mxc://example.org/y", DownloadKind::Original, None)
        .unwrap()
        .download_id
        .clone();
    q.begin(&id).unwrap();
    let err = q.complete(&id, "data:image/png;base64,AAA").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.2-forbidden-handle-scheme");
}

#[test]
fn download_retire() {
    let mut q = DownloadQueue::new(1);
    q.enqueue("mxc://example.org/z", DownloadKind::Avatar, None)
        .unwrap();
    q.retire_generation(7);
    assert!(q.is_empty());
    assert_eq!(q.session_generation(), 7);
}

#[test]
fn content_upload_mime_accepts_non_image_and_rejects_invalid() {
    assert!(parse_content_upload_mime("application/octet-stream").is_ok());
    assert!(parse_content_upload_mime("image/jpeg").is_ok());
    let invalid = parse_content_upload_mime("not-a-mime").unwrap_err();
    assert_eq!(invalid, "v-send.r-content-upload-invalid-mime");
    assert!(!invalid.contains("not-a-mime"));
    let empty = parse_content_upload_mime("").unwrap_err();
    assert_eq!(empty, "v-send.r-content-upload-invalid-mime");
}

#[test]
fn content_upload_filename_rejects_without_echo() {
    let secret = "secret.bin";
    assert_eq!(validate_content_upload_filename(secret).unwrap(), secret);
    let slash = validate_content_upload_filename("../secret.bin").unwrap_err();
    assert_eq!(slash, "v-send.r-content-upload-invalid-filename");
    assert!(!slash.contains("secret.bin"));
    let empty = validate_content_upload_filename("").unwrap_err();
    assert_eq!(empty, "v-send.r-content-upload-invalid-filename");
}
