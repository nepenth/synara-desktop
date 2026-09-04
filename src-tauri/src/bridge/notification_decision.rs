//! Desktop bridges for the Core notification decision stream through
//! `Core::command`.
//!
//! Core owns the suppress/show policy; this bridge only transports closed
//! observations in and typed readbacks out. Delivery stays in
//! `desktop_notifications.rs` via the platform facade.

use synara_core::app::notifications::{
    NativeNotificationDecideRequest, NativeNotificationDismissRequest,
    NativeNotificationFocusSetRequest, NotificationDecisionReadback,
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
) -> Result<bool, MatrixAuthCommandError> {
    let request = NativeNotificationDismissRequest { candidate_id };
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

pub(crate) async fn notification_pending_snapshot(
    core: &Core,
) -> Result<Vec<NotificationCandidate>, MatrixAuthCommandError> {
    let body = dispatch(
        core,
        "matrix_notification_pending_snapshot",
        serde_json::Value::Null,
    )
    .await?;
    #[derive(serde::Deserialize)]
    struct Wire {
        candidates: Vec<NotificationCandidate>,
    }
    serde_json::from_value::<Wire>(body)
        .map(|wire| wire.candidates)
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
