//! P6.1 — Outbound text send queue + local-echo foundation (harness).
//! P7.4 — Outbound attachment / media send queue foundation (harness).
//!
//! Tracks plain-text and attachment outbound messages with [`LocalEchoState`]
//! and session generation stamps. Attachment queue uses **media handle ids
//! only** (no file bytes). No SDK `Room::send`, no dual-backend.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p6.1-send-queue.md`
//! - `docs/matrix-rust-sdk/p7.4-attachment-send.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod attachment;
mod attachment_queue;
mod error;
mod ipc;
mod poll;
mod queue;
mod text;

pub use attachment::{
    attachment_caption, attachment_config, attachment_reply, send_room_attachment,
    validate_attachment_filename, validate_attachment_mime, SendRoomAttachmentRequest,
    MAX_ATTACHMENT_UPLOAD_BYTES,
};
pub use attachment_queue::{
    AttachmentEnqueue, AttachmentKind, AttachmentSendQueue, OutboundAttachment,
    MAX_ACTIVE_ATTACHMENTS, MAX_CAPTION_CHARS, MAX_HANDLE_CHARS,
};
pub use error::SendError;
pub use ipc::{
    MatrixPollRespondResult, MatrixSendAttachmentResult, MatrixSendPollResult,
    MatrixSendRoomAttachmentResult, MatrixSendTextResult,
};
pub use poll::{
    apply_poll_start_relations, normalize_poll, poll_response_content, poll_start_content,
    NormalizedPoll, PollSendError,
};
pub use queue::{LocalTxnId, OutboundTextMessage, SendQueue};
pub use text::{
    edit_message_content, message_content, parse_edit_event_id, parse_reply_event_id,
    parse_send_room_id, parse_thread_root_event_id, parse_transaction_id, send_message_to_room,
    validate_outbound_text_payload, validated_mentions, MAX_MATRIX_IDENTIFIER_BYTES,
    MAX_OUTBOUND_MENTION_COUNT, MAX_OUTBOUND_TEXT_PAYLOAD_BYTES,
};

/// Static marker for link / schema smoke (text + attachment queues).
pub const MATRIX_SEND_MARKER: &str = "matrix-send-queue-p6.1+attachment-p7.4";

/// Touch send-queue paths so they remain linked in non-test builds.
pub fn matrix_send_markers() -> &'static str {
    let q = SendQueue::new(0);
    let a = AttachmentSendQueue::new(0);
    debug_assert!(q.is_empty());
    debug_assert!(a.is_empty());
    debug_assert_eq!(q.active_count(), 0);
    debug_assert_eq!(AttachmentKind::Image.as_str(), "image");
    debug_assert_eq!(MATRIX_SEND_MARKER, "matrix-send-queue-p6.1+attachment-p7.4");
    MATRIX_SEND_MARKER
}

#[cfg(test)]
mod tests;
