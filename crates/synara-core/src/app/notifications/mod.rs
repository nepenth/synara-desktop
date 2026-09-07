//! P7.1 — Notification candidate index foundation.
//!
//! Pure index of privacy-filtered Synara [`NotificationCandidate`] DTOs plus
//! the account-bound [`NativeNotificationDecisionOwner`] production policy
//! over it. No OS notification posting, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.1-notifications.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod decision;
mod edit_policy;
mod error;
mod http_pusher;
mod index;
mod push_rules;
mod room_notification;

pub use decision::{
    NativeNotificationDecideRequest, NativeNotificationDecisionOwner,
    NativeNotificationDismissRequest, NativeNotificationFocusSetRequest, NotificationDecisionInput,
    NotificationDecisionKind, NotificationDecisionReadback, NotificationRoomMode,
    NotificationSuppressReason, NOTIFICATION_BODY_MAX_CHARS, NOTIFICATION_ROUTE_MAX_CHARS,
    NOTIFICATION_TITLE_MAX_CHARS,
};
pub use error::NotificationError;
pub use http_pusher::{
    delete_http_pusher, register_http_pusher, MatrixHttpPusherWriteResult, NativeHttpPusherOwner,
    MAX_APP_ID_BYTES, MAX_PUSH_KEY_BYTES,
};
pub use index::{NotificationIndex, MAX_PENDING_CANDIDATES};
pub use push_rules::{
    add_keyword, remove_keyword, set_default_room_mode, set_mention_enabled, snapshot_push_rules,
    MatrixPushRuleMentions, MatrixPushRulesSnapshot, MatrixPushRulesWriteResult,
};
pub use room_notification::{
    set_room_notification, snapshot_room_notification, snapshot_room_notifications,
    MatrixRoomNotificationSnapshot, MatrixRoomNotificationWriteResult,
    MatrixRoomNotificationsSnapshot,
};

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
