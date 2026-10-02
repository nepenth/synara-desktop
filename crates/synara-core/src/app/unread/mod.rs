//! P5.5 — Read markers, receipts, typing, and unread positioning (harness).
//!
//! Pure room read-state + open-position policy over Synara receipt DTOs and
//! room-list unread signals. Typing remains in `matrix::typing` (P6.3).
//! No SDK network, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.5-unread-positioning.md`
//! Contract: `docs/timeline-room-state-reliability-contract.md`

mod error;
mod state;

pub use error::UnreadError;
pub use state::{
    FrontierSource, OpenPositionPolicy, ReceiptPrivacy, RoomReadState, UnreadPositionStore,
    MAX_TRACKED_ROOMS,
};

#[cfg(test)]
mod tests;
