//! Unit tests for P6.1 send queue.

use super::*;
use crate::dto::LocalEchoState;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_send_markers(), MATRIX_SEND_MARKER);
}

#[test]
fn text_payload_budget_is_combined_utf8_bytes_and_shared_by_send_and_edit() {
    let at_limit = "🙂".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES / "🙂".len());
    assert_eq!(at_limit.len(), MAX_OUTBOUND_TEXT_PAYLOAD_BYTES);
    assert!(message_content(at_limit.clone(), None, None, None, false, None, None,).is_ok());

    let event_id = "$edit:example.org".parse().expect("valid event id");
    assert!(edit_message_content(at_limit, None, None, None, false, event_id,).is_ok());

    let plain_over_limit = format!(
        "{}x",
        "🙂".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES / "🙂".len())
    );
    assert_eq!(plain_over_limit.len(), MAX_OUTBOUND_TEXT_PAYLOAD_BYTES + 1);
    assert_eq!(
        message_content(plain_over_limit, None, None, None, false, None, None,)
            .expect_err("oversized plain body must be rejected"),
        "d0.4-send-text-payload-too-large"
    );

    let body = "fallback";
    let formatted_at_limit = "x".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES - body.len());
    assert!(message_content(
        body.to_owned(),
        None,
        Some(formatted_at_limit),
        None,
        false,
        None,
        None,
    )
    .is_ok());
    let formatted_over_limit = "x".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES - body.len() + 1);
    assert_eq!(
        message_content(
            body.to_owned(),
            None,
            Some(formatted_over_limit),
            None,
            false,
            None,
            None,
        )
        .expect_err("oversized combined body and formatted body must be rejected"),
        "d0.4-send-text-payload-too-large"
    );
}

#[test]
fn mentions_are_bounded_before_parsing_and_set_allocation() {
    let valid_id_at_limit = format!("@{}:x", "a".repeat(MAX_MATRIX_IDENTIFIER_BYTES - 3));
    assert_eq!(valid_id_at_limit.len(), MAX_MATRIX_IDENTIFIER_BYTES);
    assert!(validated_mentions(Some(vec![valid_id_at_limit]), false).is_ok());

    let oversized_id = format!("@{}:x", "a".repeat(MAX_MATRIX_IDENTIFIER_BYTES - 2));
    assert_eq!(oversized_id.len(), MAX_MATRIX_IDENTIFIER_BYTES + 1);
    assert_eq!(
        validated_mentions(Some(vec![oversized_id]), false)
            .expect_err("oversized mention identifier must fail before parsing"),
        "v-send.4-mention-user-id-too-long"
    );

    let duplicate = "@alice:example.org".to_owned();
    assert!(validated_mentions(
        Some(vec![duplicate.clone(); MAX_OUTBOUND_MENTION_COUNT]),
        false
    )
    .is_ok());
    assert_eq!(
        validated_mentions(Some(vec![duplicate; MAX_OUTBOUND_MENTION_COUNT + 1]), false)
            .expect_err("raw mention count must be bounded before deduplication"),
        "v-send.4-too-many-mentions"
    );
}

#[test]
fn outbound_matrix_identifiers_reject_overlong_values_before_parsing() {
    let room_at_limit = format!("!{}", "r".repeat(MAX_MATRIX_IDENTIFIER_BYTES - 1));
    assert_eq!(room_at_limit.len(), MAX_MATRIX_IDENTIFIER_BYTES);
    assert!(parse_send_room_id(&room_at_limit).is_ok());

    let room_over_limit = format!("!{}", "r".repeat(MAX_MATRIX_IDENTIFIER_BYTES));
    assert_eq!(
        parse_send_room_id(&room_over_limit),
        Err("d0.4-send-invalid-room-id")
    );

    let event_over_limit = format!("${}", "e".repeat(MAX_MATRIX_IDENTIFIER_BYTES));
    assert_eq!(
        parse_edit_event_id(&event_over_limit),
        Err("v-send.r-edit-invalid-event-id")
    );
    assert_eq!(
        parse_reply_event_id(Some(event_over_limit.clone())),
        Err("d0.4-send-invalid-reply-event-id")
    );
    assert_eq!(
        parse_thread_root_event_id(Some(event_over_limit)),
        Err("v-send.5-invalid-thread-root-event-id")
    );
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

#[test]
fn attachment_filename_and_mime_reject_without_echo() {
    let secret = "secret.bin";
    assert_eq!(validate_attachment_filename(secret).unwrap(), secret);
    let slash = validate_attachment_filename("../secret.bin").unwrap_err();
    assert_eq!(slash, "v-send.1-attachment-invalid-filename");
    assert!(!slash.contains("secret.bin"));
    let mime = validate_attachment_mime("application/octet-stream").unwrap();
    assert_eq!(mime.essence_str(), "application/octet-stream");
    let invalid = validate_attachment_mime("not-a-mime").unwrap_err();
    assert_eq!(invalid, "v-send.1-attachment-invalid-mime");
    assert!(!invalid.contains("not-a-mime"));
}
