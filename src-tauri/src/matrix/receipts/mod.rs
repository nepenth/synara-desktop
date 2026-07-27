//! P6.2 — Receipt index foundation (harness).
//!
//! Pure projection of Synara [`Receipt`] DTOs per room/user/type. No SDK
//! `send_single_receipt`, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.2-receipts.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::ReceiptError;
pub use index::ReceiptIndex;

/// Static marker for link / schema smoke.
pub const MATRIX_RECEIPTS_MARKER: &str = "matrix-receipts-p6.2";

/// Touch receipt paths so they remain linked in non-test builds.
pub fn matrix_receipts_markers() -> &'static str {
    let idx = ReceiptIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.room_count(), 0);
    debug_assert_eq!(MATRIX_RECEIPTS_MARKER, "matrix-receipts-p6.2");
    MATRIX_RECEIPTS_MARKER
}

#[cfg(test)]
mod tests;
