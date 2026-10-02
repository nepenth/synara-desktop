//! P7.1 — Notification candidate index foundation.
//!
//! Pure index of privacy-filtered Synara [`NotificationCandidate`] DTOs plus
//! the account-bound [`NativeNotificationDecisionOwner`] production policy
//! over it. No OS notification posting, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.1-notifications.md`

mod decision;
mod edit_policy;
mod error;
mod http_pusher;
mod inbox;
mod index;
mod observation;
mod push_rules;
mod room_notification;

pub use decision::{
    NativeNotificationDecideRequest, NativeNotificationDecisionOwner,
    NativeNotificationDismissRequest, NativeNotificationFocusSetRequest, NotificationDecisionInput,
    NotificationDecisionKind, NotificationDecisionReadback, NotificationDeliveryLedger,
    NotificationDeliveryOutcome, NotificationPushEvaluation, NotificationSuppressReason,
    NOTIFICATION_BODY_MAX_CHARS, NOTIFICATION_ROUTE_MAX_CHARS, NOTIFICATION_TITLE_MAX_CHARS,
};
pub use error::NotificationError;
pub use http_pusher::{
    delete_http_pusher, register_http_pusher, MatrixHttpPusherWriteResult, NativeHttpPusherOwner,
    MAX_APP_ID_BYTES, MAX_PUSH_KEY_BYTES,
};
pub use inbox::{
    fetch_inbox_notifications, MatrixInboxNotificationsPage, MatrixInboxNotificationsRequest,
};
pub use index::{NotificationIndex, MAX_PENDING_CANDIDATES};
pub use observation::{
    project_observation, NativeNotificationObservation, NativeNotificationObservationOwner,
    NotificationObservationEmit, NOTIFICATION_OBSERVATION_WINDOW_MS, NOTIFICATION_OBSERVED_EVENT,
};
pub use push_rules::{
    add_keyword, remove_keyword, set_default_room_mode, set_mention_enabled, snapshot_push_rules,
    MatrixPushRuleMentions, MatrixPushRulesSnapshot, MatrixPushRulesWriteResult,
};
pub use room_notification::{
    set_room_notification, snapshot_room_notification, snapshot_room_notifications,
    MatrixRoomNotificationSnapshot, MatrixRoomNotificationWriteResult,
    MatrixRoomNotificationsSnapshot,
};

#[cfg(test)]
mod tests;
