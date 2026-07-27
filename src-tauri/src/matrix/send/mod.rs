//! P6.1 — Outbound send queue + local-echo foundation (harness).
//!
//! Tracks plain-text outbound messages with [`LocalEchoState`] and session
//! generation stamps. No SDK `Room::send`, no production Tauri commands, no
//! dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.1-send-queue.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod queue;

pub use error::SendError;
pub use queue::{LocalTxnId, OutboundTextMessage, SendQueue};

/// Static marker for link / schema smoke.
pub const MATRIX_SEND_MARKER: &str = "matrix-send-queue-p6.1";

/// Touch send-queue paths so they remain linked in non-test builds.
pub fn matrix_send_markers() -> &'static str {
    let q = SendQueue::new(0);
    debug_assert!(q.is_empty());
    debug_assert_eq!(q.active_count(), 0);
    debug_assert_eq!(MATRIX_SEND_MARKER, "matrix-send-queue-p6.1");
    MATRIX_SEND_MARKER
}

#[cfg(test)]
mod tests;
