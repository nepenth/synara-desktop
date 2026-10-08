//! Desktop bridges for the Core notification decision stream through
//! `Core::command`.
//!
//! Core owns the suppress/show policy; this bridge only transports closed
//! observations in and typed readbacks out. Delivery stays in
//! `desktop_notifications.rs` via the platform facade.

use synara_core::app::notifications::{
    NativeNotificationDecideRequest, NativeNotificationDismissRequest,
    NativeNotificationFocusSetRequest, NotificationDecisionReadback, NotificationDeliveryOutcome,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn notification_focus_set(
    core: &Core,
    room_id: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    core.notification_focus_set(NativeNotificationFocusSetRequest { room_id })
        .await
        .map_err(map_notification_core_error)?;
    Ok(())
}

pub(crate) async fn notification_decide(
    core: &Core,
    request: NativeNotificationDecideRequest,
) -> Result<NotificationDecisionReadback, MatrixAuthCommandError> {
    core.notification_decide(request)
        .await
        .map_err(map_notification_core_error)
}

pub(crate) async fn notification_dismiss(
    core: &Core,
    candidate_id: String,
    outcome: Option<NotificationDeliveryOutcome>,
) -> Result<bool, MatrixAuthCommandError> {
    let request = NativeNotificationDismissRequest {
        candidate_id,
        outcome,
    };
    core.notification_dismiss(request)
        .await
        .map(|result| result.dismissed)
        .map_err(map_notification_core_error)
}

/// Typed readback for `matrix_notification_pending_snapshot`: the pending
/// Core-decided candidates plus the identifier-free per-session delivery
/// ledger Core echoes alongside them. The ledger is the live observable for
/// OS delivery receipts; the shell must not drop it on the way out.
pub use synara_core::core_api::MatrixNotificationPendingSnapshot as NotificationPendingSnapshot;

pub(crate) async fn notification_pending_snapshot(
    core: &Core,
) -> Result<NotificationPendingSnapshot, MatrixAuthCommandError> {
    core.notification_pending_snapshot()
        .await
        .map_err(map_notification_core_error)
}

fn map_notification_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error.diagnostic_id.as_deref().unwrap_or("v-notify.failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native notification decision request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native notification decision stream is unavailable.",
            diagnostic,
        ),
    }
}
