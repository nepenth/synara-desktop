//! Desktop bridge for MSC4426 status snapshot/set/clear through `Core::command`.
//!
//! Core owns `NativeUserStatusOwner` after the shell attaches it. React
//! invokes `matrix_user_status_snapshot` / `set` / `clear`. This never calls
//! `set_call`.

use synara_core::app::user_status::{NativeUserStatusSnapshot, NativeUserStatusWriteResult};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const USER_STATUS_SNAPSHOT_COMMAND: &str = "matrix_user_status_snapshot";
const USER_STATUS_SET_COMMAND: &str = "matrix_user_status_set";
const USER_STATUS_CLEAR_COMMAND: &str = "matrix_user_status_clear";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn user_status_snapshot(
    core: &Core,
    user_id: String,
) -> Result<NativeUserStatusSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: USER_STATUS_SNAPSHOT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "userId": user_id }),
        })
        .await
        .map_err(map_user_status_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| user_status_response_error())
}

pub(crate) async fn user_status_set(
    core: &Core,
    emoji: String,
    text: String,
) -> Result<NativeUserStatusWriteResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: USER_STATUS_SET_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "emoji": emoji, "text": text }),
        })
        .await
        .map_err(map_user_status_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| user_status_response_error())
}

pub(crate) async fn user_status_clear(
    core: &Core,
) -> Result<NativeUserStatusWriteResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: USER_STATUS_CLEAR_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_user_status_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| user_status_response_error())
}

fn map_user_status_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("p2-user-status-snapshot-no-session"),
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic_id = match error.diagnostic_id.as_deref() {
                Some("v-user-status-unsupported") => "v-user-status-unsupported",
                Some("v-user-status-emoji-cap") => "v-user-status-emoji-cap",
                Some("v-user-status-text-cap") => "v-user-status-text-cap",
                Some("v-user-status-invalid-user-id") => "v-user-status-invalid-user-id",
                _ => "p2-user-status-set-invalid-payload",
            };
            let message = if diagnostic_id == "v-user-status-unsupported" {
                "This homeserver does not support user status."
            } else {
                "The native user status request is invalid."
            };
            MatrixAuthCommandError::new("InvalidRequest", message, diagnostic_id)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native user status is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("p2-user-status-snapshot-serialization-failed"),
        ),
    }
}

fn user_status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native user status is unavailable.",
        "p2-user-status-snapshot-serialization-failed",
    )
}
