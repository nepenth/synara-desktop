//! P7.1 — Notification candidate index foundation.
//!
//! Pure index of privacy-filtered Synara [`NotificationCandidate`] DTOs plus
//! the account-bound [`NativeNotificationDecisionOwner`] production policy
//! over it. No OS notification posting, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.1-notifications.md`

mod agent_preferences;
pub use agent_preferences::*;
#[cfg(feature = "full-app")]
mod decision;
#[cfg(feature = "full-app")]
mod edit_policy;
#[cfg(feature = "full-app")]
mod error;
#[cfg(feature = "full-app")]
mod http_pusher;
#[cfg(feature = "full-app")]
mod inbox;
#[cfg(feature = "full-app")]
mod index;
mod nse_error;
pub use nse_error::{nse_notification_error_code, nse_notification_initialization_error_code};
#[cfg(feature = "full-app")]
mod observation;
#[cfg(feature = "full-app")]
mod push_rules;
#[cfg(feature = "full-app")]
mod room_notification;

#[cfg(feature = "full-app")]
pub use decision::{
    NativeNotificationDecideRequest, NativeNotificationDecisionOwner,
    NativeNotificationDismissRequest, NativeNotificationFocusSetRequest, NotificationDecisionInput,
    NotificationDecisionKind, NotificationDecisionReadback, NotificationDeliveryLedger,
    NotificationDeliveryOutcome, NotificationPushEvaluation, NotificationSuppressReason,
    NOTIFICATION_BODY_MAX_CHARS, NOTIFICATION_ROUTE_MAX_CHARS, NOTIFICATION_TITLE_MAX_CHARS,
};
#[cfg(feature = "full-app")]
pub use error::NotificationError;
#[cfg(feature = "full-app")]
pub use http_pusher::{
    delete_http_pusher, register_http_pusher, MatrixHttpPusherWriteResult, NativeHttpPusherOwner,
    MAX_APP_ID_BYTES, MAX_PUSH_KEY_BYTES,
};
#[cfg(feature = "full-app")]
pub use inbox::{
    fetch_inbox_notifications, MatrixInboxNotificationsPage, MatrixInboxNotificationsRequest,
};
#[cfg(feature = "full-app")]
pub use index::{NotificationIndex, MAX_PENDING_CANDIDATES};
#[cfg(feature = "full-app")]
pub use observation::{
    project_observation, NativeNotificationObservation, NativeNotificationObservationOwner,
    NotificationObservationEmit, NOTIFICATION_OBSERVATION_WINDOW_MS, NOTIFICATION_OBSERVED_EVENT,
};
#[cfg(feature = "full-app")]
pub use push_rules::{
    add_keyword, remove_keyword, set_default_room_mode, set_mention_enabled, snapshot_push_rules,
    MatrixPushRuleMentions, MatrixPushRulesSnapshot, MatrixPushRulesWriteResult,
};
#[cfg(feature = "full-app")]
pub use room_notification::{
    set_room_notification, snapshot_room_notification, snapshot_room_notifications,
    MatrixRoomNotificationSnapshot, MatrixRoomNotificationWriteResult,
    MatrixRoomNotificationsSnapshot,
};

#[cfg(all(test, feature = "full-app"))]
mod tests;
