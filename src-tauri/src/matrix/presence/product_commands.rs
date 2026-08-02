use super::*;
use crate::matrix::presence::{NativePresenceSnapshotResult, NativePresenceSubscription};

#[tauri::command]
pub async fn matrix_presence_snapshot(
    state: State<'_, MatrixAuthState>,
    user_id: String,
) -> Result<NativePresenceSnapshotResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    active
        .presence
        .snapshot(&user_id)
        .await
        .map_err(map_presence_error)
}

#[tauri::command]
pub async fn matrix_presence_subscribe(
    state: State<'_, MatrixAuthState>,
    user_id: String,
) -> Result<NativePresenceSubscription, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    active
        .presence
        .subscribe(&user_id)
        .await
        .map_err(map_presence_error)
}

#[tauri::command]
pub async fn matrix_presence_unsubscribe(
    state: State<'_, MatrixAuthState>,
    subscription_id: String,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    active
        .presence
        .unsubscribe(&subscription_id)
        .await
        .map_err(map_presence_error)
}

pub(super) fn map_presence_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-presence-invalid-user-id" | "v-presence-invalid-subscription-id" => (
            "InvalidRequest",
            "The native Matrix presence request is invalid.",
        ),
        "v-presence-user-owner-missing" => ("Forbidden", "No native Matrix session is active."),
        _ => ("Unknown", "Native Matrix presence is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
