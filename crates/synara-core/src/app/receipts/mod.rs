//! P6.2 — Receipt index foundation (harness).
//!
//! Pure projection of Synara [`Receipt`] DTOs per room/user/type. No SDK
//! `send_single_receipt`, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.2-receipts.md`

mod error;
mod index;

pub use error::ReceiptError;
pub use index::ReceiptIndex;

#[cfg(test)]
mod tests;
