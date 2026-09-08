//! Desktop bridges for the Core notification decision stream through
//! `Core::command`.
//!
//! Core owns the suppress/show policy; this bridge only transports closed
//! observations in and typed readbacks out. Delivery stays in
//! `desktop_notifications.rs` via the platform facade.

use synara_core::app::notifications::{
    NativeNotificationDecideRequest, NativeNotificationDismissRequest,
    NativeNotificationFocusSetRequest, NotificationDecisionReadback, NotificationDeliveryLedger,
    NotificationDeliveryOutcome,
};
use synara_core::dto::NotificationCandidate;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn notification_focus_set(
    core: &Core,
    room_id: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    let request = NativeNotificationFocusSetRequest { room_id };
    let payload = serde_json::to_value(request).map_err(|_| notification_response_error())?;
    dispatch(core, "matrix_notification_focus_set", payload).await?;
    Ok(())
}

pub(crate) async fn notification_decide(
    core: &Core,
    request: NativeNotificationDecideRequest,
) -> Result<NotificationDecisionReadback, MatrixAuthCommandError> {
    let payload = serde_json::to_value(request).map_err(|_| notification_response_error())?;
    let body = dispatch(core, "matrix_notification_decide", payload).await?;
    serde_json::from_value(body).map_err(|_| notification_response_error())
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
    let payload = serde_json::to_value(request).map_err(|_| notification_response_error())?;
    let body = dispatch(core, "matrix_notification_dismiss", payload).await?;
    #[derive(serde::Deserialize)]
    struct Wire {
        dismissed: bool,
    }
    serde_json::from_value::<Wire>(body)
        .map(|wire| wire.dismissed)
        .map_err(|_| notification_response_error())
}

/// Typed readback for `matrix_notification_pending_snapshot`: the pending
/// Core-decided candidates plus the identifier-free per-session delivery
/// ledger Core echoes alongside them. The ledger is the live observable for
/// OS delivery receipts; the shell must not drop it on the way out.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPendingSnapshot {
    pub candidates: Vec<NotificationCandidate>,
    pub delivery: NotificationDeliveryLedger,
}

pub(crate) async fn notification_pending_snapshot(
    core: &Core,
) -> Result<NotificationPendingSnapshot, MatrixAuthCommandError> {
    let body = dispatch(
        core,
        "matrix_notification_pending_snapshot",
        serde_json::Value::Null,
    )
    .await?;
    serde_json::from_value::<NotificationPendingSnapshot>(body)
        .map_err(|_| notification_response_error())
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map(|response| response.payload)
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

fn notification_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native notification decision stream is unavailable.",
        "v-notify.failed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_snapshot_readback_carries_the_delivery_ledger() {
        // Core echoes `{ candidates, delivery }`; the shell must forward the
        // ledger unchanged so a live OS receipt is observable through the
        // Tauri command, not only through `Core::command`.
        let snapshot: NotificationPendingSnapshot = serde_json::from_value(serde_json::json!({
            "candidates": [],
            "delivery": { "delivered": 2, "failed": 1, "unreported": 0 },
        }))
        .expect("Core pending snapshot deserializes");
        assert!(snapshot.candidates.is_empty());
        assert_eq!(
            snapshot.delivery,
            NotificationDeliveryLedger {
                delivered: 2,
                failed: 1,
                unreported: 0,
            }
        );
        let wire = serde_json::to_value(&snapshot).expect("snapshot serializes");
        assert_eq!(wire["delivery"]["delivered"], 2);
        assert_eq!(wire["delivery"]["failed"], 1);
    }

    #[test]
    fn pending_snapshot_readback_without_ledger_fails_closed() {
        let result = serde_json::from_value::<NotificationPendingSnapshot>(serde_json::json!({
            "candidates": [],
        }));
        assert!(result.is_err());
    }
}
