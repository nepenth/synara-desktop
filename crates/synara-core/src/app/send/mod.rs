//! P6.1 — Outbound text send queue + local-echo foundation (harness).
//! P7.4 — Outbound attachment / media send queue foundation (harness).
//!
//! Product `SendQueue` / `AttachmentSendQueue` project `RoomSendQueue` /
//! `SendHandle` state (Sending / Sent / Failed / Wedged / Cancelled).
//! Network I/O is `RoomSendQueue::send` / `send_attachment`. No
//! `AttachmentConfig.extra_content`. No dual-backend.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p6.1-send-queue.md`
//! - `docs/matrix-rust-sdk/p7.4-attachment-send.md`

mod attachment;
mod attachment_queue;
mod error;
mod ipc;
mod poll;
mod queue;
mod recovery;
mod room_queue;
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
pub use recovery::{send_queue_retry_delay, spawn_send_queue_recovery};
pub use room_queue::{
    abort_queued_send, enqueue_attachment_via_room_queue, enqueue_event_via_room_queue,
    queued_send_is_wedged, queued_send_outcome, redact_via_room_queue, reenable_queued_send,
    send_attachment_via_room_queue, send_event_via_room_queue, unwedge_queued_send,
    wait_for_queued_send, QueuedSendAck, QueuedSendError, QueuedSendOutcome, QueuedSendSession,
};
pub use text::{
    edit_message_content, message_content, parse_edit_event_id, parse_reply_event_id,
    parse_send_room_id, parse_thread_root_event_id, parse_transaction_id, send_message_to_room,
    validate_outbound_text_payload, validated_mentions, MAX_MATRIX_IDENTIFIER_BYTES,
    MAX_OUTBOUND_MENTION_COUNT, MAX_OUTBOUND_TEXT_PAYLOAD_BYTES,
};

#[cfg(test)]
mod tests;
