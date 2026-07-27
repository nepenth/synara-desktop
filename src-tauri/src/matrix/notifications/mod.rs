//! P7.1 — Notification candidate index foundation (harness).
//!
//! Pure index of privacy-filtered Synara [`NotificationCandidate`] DTOs.
//! No OS notification posting, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.1-notifications.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::NotificationError;
pub use index::{NotificationIndex, MAX_PENDING_CANDIDATES};

/// Static marker for link / schema smoke.
pub const MATRIX_NOTIFICATIONS_MARKER: &str = "matrix-notifications-p7.1";

/// Touch notification paths so they remain linked in non-test builds.
pub fn matrix_notifications_markers() -> &'static str {
    let idx = NotificationIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.len(), 0);
    debug_assert_eq!(MATRIX_NOTIFICATIONS_MARKER, "matrix-notifications-p7.1");
    MATRIX_NOTIFICATIONS_MARKER
}

#[cfg(test)]
mod tests;
