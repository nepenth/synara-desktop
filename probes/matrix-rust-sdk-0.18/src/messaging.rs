//! Message send, state send, redaction, receipts, and typing probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::Room;
use matrix_sdk::room::futures::SendMessageLikeEvent;
use matrix_sdk::ruma::api::client::receipt::create_receipt::v3::ReceiptType;
use matrix_sdk::ruma::events::receipt::ReceiptThread;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::events::room::name::RoomNameEventContent;
use matrix_sdk::ruma::events::room::redaction::RoomRedactionEventContent;
use matrix_sdk::ruma::{EventId, OwnedEventId, OwnedTransactionId};
use matrix_sdk_ui::Timeline;

/// P0.3b-room-send — `Room::send` → `SendMessageLikeEvent`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub fn send`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_send() {
    fn _shape(room: &Room, content: RoomMessageEventContent) -> SendMessageLikeEvent<'_> {
        room.send(content)
    }
    let _ = _shape;
}

/// P0.3b-room-send-state-event — `Room::send_state_event`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn send_state_event`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_send_state_event() {
    async fn _shape(
        room: &Room,
        content: RoomNameEventContent,
    ) -> matrix_sdk::Result<matrix_sdk::ruma::api::client::state::send_state_event::v3::Response>
    {
        room.send_state_event(content).await
    }
    let _ = _shape;
}

/// P0.3b-room-redact — `Room::redact`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn redact`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_redact() {
    async fn _shape(
        room: &Room,
        event_id: &EventId,
        reason: Option<&str>,
        txn_id: Option<OwnedTransactionId>,
    ) -> matrix_sdk::HttpResult<matrix_sdk::ruma::api::client::redact::redact_event::v3::Response>
    {
        room.redact(event_id, reason, txn_id).await
    }
    let _ = _shape;
    // Keep RoomRedactionEventContent name-checked as related public ruma type.
    let _ = std::any::type_name::<RoomRedactionEventContent>();
}

/// P0.3b-room-typing-notice — `Room::typing_notice`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn typing_notice`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_typing_notice() {
    async fn _shape(room: &Room, typing: bool) -> matrix_sdk::Result<()> {
        room.typing_notice(typing).await
    }
    let _ = _shape;
}

/// P0.3b-room-send-single-receipt — `Room::send_single_receipt`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn send_single_receipt`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_send_single_receipt() {
    async fn _shape(
        room: &Room,
        receipt_type: ReceiptType,
        thread: ReceiptThread,
        event_id: OwnedEventId,
    ) -> matrix_sdk::Result<()> {
        room.send_single_receipt(receipt_type, thread, event_id)
            .await
    }
    let _ = _shape;
}

/// P0.3b-timeline-mark-as-read — `Timeline::mark_as_read`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/mod.rs` (`pub async fn mark_as_read`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_mark_as_read() {
    async fn _shape(timeline: &Timeline, receipt_type: ReceiptType) -> matrix_sdk::Result<bool> {
        timeline.mark_as_read(receipt_type).await
    }
    let _ = _shape;
}

/// Run every messaging probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_room_send();
    probe_room_send_state_event();
    probe_room_redact();
    probe_room_typing_notice();
    probe_room_send_single_receipt();
    probe_timeline_mark_as_read();
}
