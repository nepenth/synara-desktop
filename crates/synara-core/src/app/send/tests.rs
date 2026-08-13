//! Unit tests for P6.1 send queue.

use super::*;
use crate::dto::LocalEchoState;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_send_markers(), MATRIX_SEND_MARKER);
}

#[test]
fn enqueue_mark_sent_flow() {
    let mut q = SendQueue::new(4);
    let item = q.enqueue_text("!room:example.org", "hello").unwrap();
    assert_eq!(item.session_generation, 4);
    assert_eq!(item.state, LocalEchoState::Sending);
    assert!(item.local_txn_id.starts_with("local-txn-"));
    let id = item.local_txn_id.clone();
    assert_eq!(q.active_count(), 1);

    q.mark_sent(&id).unwrap();
    let done = q.get(&id).unwrap();
    assert_eq!(done.state, LocalEchoState::Sent);
    assert_eq!(q.active_count(), 0);
}

#[test]
fn mark_failed_retry_cancel() {
    let mut q = SendQueue::new(1);
    let id = q
        .enqueue_text("!r:example.org", "ping")
        .unwrap()
        .local_txn_id
        .clone();
    q.mark_failed(&id, "p6.1-network-failed").unwrap();
    let f = q.get(&id).unwrap();
    assert_eq!(f.state, LocalEchoState::Failed);
    assert_eq!(f.failure_diagnostic_id, Some("p6.1-network-failed"));
    assert!(!f.failure_diagnostic_id.unwrap().contains("access_token"));

    q.retry(&id).unwrap();
    assert_eq!(q.get(&id).unwrap().state, LocalEchoState::Sending);

    q.cancel(&id).unwrap();
    assert_eq!(q.get(&id).unwrap().state, LocalEchoState::Cancelled);
}

#[test]
fn invalid_room_and_empty_body() {
    let mut q = SendQueue::new(1);
    let err = q.enqueue_text("not-a-room", "hi").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.1-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    let err = q.enqueue_text("!r:example.org", "").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.1-empty-body");
}

#[test]
fn list_for_room_and_prune() {
    let mut q = SendQueue::new(1);
    let a = q
        .enqueue_text("!a:example.org", "one")
        .unwrap()
        .local_txn_id
        .clone();
    let _b = q.enqueue_text("!b:example.org", "two").unwrap();
    assert_eq!(q.list_for_room("!a:example.org").len(), 1);
    assert_eq!(q.len(), 2);

    q.mark_sent(&a).unwrap();
    let pruned = q.prune_terminal();
    assert_eq!(pruned, 1);
    assert_eq!(q.len(), 1);
    assert!(q.get(&a).is_none());
}

#[test]
fn retire_generation_cancels_sending() {
    let mut q = SendQueue::new(1);
    let id = q
        .enqueue_text("!r:example.org", "x")
        .unwrap()
        .local_txn_id
        .clone();
    q.retire_generation(2);
    assert_eq!(q.session_generation(), 2);
    let item = q.get(&id).unwrap();
    assert_eq!(item.state, LocalEchoState::Cancelled);
    assert_eq!(
        item.failure_diagnostic_id,
        Some("p6.1-stale-generation-cancelled")
    );
    assert_eq!(item.session_generation, 2);
}

#[test]
fn mark_sent_invalid_when_failed() {
    let mut q = SendQueue::new(1);
    let id = q
        .enqueue_text("!r:example.org", "x")
        .unwrap()
        .local_txn_id
        .clone();
    q.mark_failed(&id, "p6.1-x").unwrap();
    let err = q.mark_sent(&id).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.1-mark-sent-invalid-state");
}

#[test]
fn clear_wipes_queue() {
    let mut q = SendQueue::new(1);
    q.enqueue_text("!r:example.org", "x").unwrap();
    q.clear();
    assert!(q.is_empty());
}

#[test]
fn attachment_enqueue_sent() {
    let mut q = AttachmentSendQueue::new(2);
    let id = q
        .enqueue(AttachmentEnqueue {
            room_id: "!r:example.org".into(),
            kind: AttachmentKind::Image,
            media_handle_id: "mxc://example.org/abc".into(),
            file_name: Some("photo.jpg".into()),
            caption: Some("hi".into()),
            mime_type: Some("image/jpeg".into()),
            size_bytes: Some(1024),
        })
        .unwrap()
        .local_txn_id
        .clone();
    assert_eq!(q.get(&id).unwrap().state, LocalEchoState::Sending);
    q.mark_sent(&id).unwrap();
    assert_eq!(q.get(&id).unwrap().state, LocalEchoState::Sent);
}

#[test]
fn attachment_fail_retry_cancel_prune() {
    let mut q = AttachmentSendQueue::new(1);
    let id = q
        .enqueue(AttachmentEnqueue {
            room_id: "!r:example.org".into(),
            kind: AttachmentKind::File,
            media_handle_id: "upload-1".into(),
            file_name: None,
            caption: None,
            mime_type: None,
            size_bytes: Some(10),
        })
        .unwrap()
        .local_txn_id
        .clone();
    q.mark_failed(&id, "p7.4-network-failed").unwrap();
    q.retry(&id).unwrap();
    q.cancel(&id).unwrap();
    assert_eq!(q.prune_terminal(), 1);
    assert!(q.is_empty());
}

#[test]
fn attachment_forbids_data_and_tokens() {
    let mut q = AttachmentSendQueue::new(1);
    let err = q
        .enqueue(AttachmentEnqueue {
            room_id: "!r:example.org".into(),
            kind: AttachmentKind::Image,
            media_handle_id: "data:image/png;base64,AAA".into(),
            file_name: None,
            caption: None,
            mime_type: None,
            size_bytes: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.4-forbidden-handle-scheme");
    let err = q
        .enqueue(AttachmentEnqueue {
            room_id: "!r:example.org".into(),
            kind: AttachmentKind::File,
            media_handle_id: "ok".into(),
            file_name: None,
            caption: Some("leaked access_token=x".into()),
            mime_type: None,
            size_bytes: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.4-forbidden-caption");
}

#[test]
fn attachment_kinds() {
    for k in AttachmentKind::ALL {
        assert!(!k.as_str().is_empty());
    }
}
