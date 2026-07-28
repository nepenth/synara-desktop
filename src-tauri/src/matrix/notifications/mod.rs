//! P7.1/P9.2 — Notification index and candidate stream foundations (harness).
//!
//! Pure index of privacy-filtered Synara [`NotificationCandidate`] DTOs.
//! The P9.2 stream retains identifiers and classification only.
//! No OS notification posting, no production Tauri commands, no dual-backend.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p7.1-notifications.md`
//! - `docs/matrix-rust-sdk/p9.2-notification-stream.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod stream;

pub use error::NotificationError;
pub use index::{NotificationIndex, MAX_PENDING_CANDIDATES};
pub use stream::{
    Candidate, CandidateKind, NotificationCandidateStream, MAX_NOTIFICATION_STREAM_CANDIDATES,
};

/// Static marker for link / schema smoke.
pub const MATRIX_NOTIFICATIONS_MARKER: &str = "matrix-notifications-p7.1+notification-stream-p9.2";

/// Touch notification paths so they remain linked in non-test builds.
pub fn matrix_notifications_markers() -> &'static str {
    let idx = NotificationIndex::new(0);
    let stream = NotificationCandidateStream::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.len(), 0);
    debug_assert!(stream.is_empty());
    debug_assert_eq!(stream.len(), 0);
    debug_assert_eq!(
        MATRIX_NOTIFICATIONS_MARKER,
        "matrix-notifications-p7.1+notification-stream-p9.2"
    );
    MATRIX_NOTIFICATIONS_MARKER
}

#[cfg(test)]
mod tests;
