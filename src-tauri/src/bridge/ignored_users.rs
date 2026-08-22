//! Desktop bridges for the ignored-user list through `Core::command`.

use synara_core::app::user_profile::{MatrixIgnoredUsersSnapshot, MatrixIgnoredUsersWriteResult};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn ignored_users_snapshot(
    core: &Core,
) -> Result<MatrixIgnoredUsersSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_ignored_users_snapshot",
        serde_json::Value::Null,
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| ignored_response_error())
}

pub(crate) async fn ignored_users_ignore(
    core: &Core,
    user_id: String,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_ignored_users_ignore",
        serde_json::json!({ "userId": user_id }),
    )
    .await?;
    parse_write(payload)
}

pub(crate) async fn ignored_users_unignore(
    core: &Core,
    user_id: String,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_ignored_users_unignore",
        serde_json::json!({ "userId": user_id }),
    )
    .await?;
    parse_write(payload)
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
    .map_err(map_ignored_core_error)
}

fn parse_write(
    payload: serde_json::Value,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    struct Wire {
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| ignored_response_error())?;
    if wire.status != "ok" {
        return Err(ignored_response_error());
    }
    Ok(MatrixIgnoredUsersWriteResult { status: "ok" })
}

fn map_ignored_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-profile.ignore-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native ignored-user request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native ignored-user list is unavailable.",
            diagnostic,
        ),
    }
}

fn ignored_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native ignored-user list is unavailable.",
        "v-profile.ignore-sdk-failed",
    )
}
