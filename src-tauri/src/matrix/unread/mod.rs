//! P5.5 — Read markers, receipts, typing, and unread positioning (harness).
//!
//! Pure room read-state + open-position policy over Synara receipt DTOs and
//! room-list unread signals. Typing remains in `matrix::typing` (P6.3).
//! No SDK network, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.5-unread-positioning.md`
//! Contract: `docs/timeline-room-state-reliability-contract.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod state;

pub use error::UnreadError;
pub use state::{
    FrontierSource, OpenPositionPolicy, ReceiptPrivacy, RoomReadState, UnreadPositionStore,
    MAX_TRACKED_ROOMS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_UNREAD_MARKER: &str = "matrix-unread-positioning-p5.5";

/// Touch unread positioning paths so they remain linked in non-test builds.
pub fn matrix_unread_markers() -> &'static str {
    let store = UnreadPositionStore::new(0);
    debug_assert!(store.is_empty());
    debug_assert_eq!(FrontierSource::FullyRead.as_str(), "fully_read");
    debug_assert_eq!(ReceiptPrivacy::Private.as_str(), "private");
    debug_assert_eq!(MATRIX_UNREAD_MARKER, "matrix-unread-positioning-p5.5");
    MATRIX_UNREAD_MARKER
}

#[cfg(test)]
mod tests;
