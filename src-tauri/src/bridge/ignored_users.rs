//! Desktop bridges for the ignored-user list through `Core::command`.

use synara_core::app::user_profile::{MatrixIgnoredUsersSnapshot, MatrixIgnoredUsersWriteResult};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn ignored_users_snapshot(
    core: &Core,
) -> Result<MatrixIgnoredUsersSnapshot, MatrixAuthCommandError> {
    let payload = core
        .ignored_users_snapshot()
        .await
        .map_err(map_ignored_core_error)?;
    Ok(payload)
}

pub(crate) async fn ignored_users_ignore(
    core: &Core,
    user_id: String,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixAuthCommandError> {
    let payload = core
        .ignored_users_ignore(synara_core::core_api::MatrixIgnoredUsersUserRequest { user_id })
        .await
        .map_err(map_ignored_core_error)?;
    Ok(payload)
}

pub(crate) async fn ignored_users_unignore(
    core: &Core,
    user_id: String,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixAuthCommandError> {
    let payload = core
        .ignored_users_unignore(synara_core::core_api::MatrixIgnoredUsersUserRequest { user_id })
        .await
        .map_err(map_ignored_core_error)?;
    Ok(payload)
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
