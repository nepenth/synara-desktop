//! Desktop bridges for own display-name / avatar reads and writes through `Core::command`.

use synara_core::app::user_profile::{MatrixOwnProfile, MatrixProfileWriteResult};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn set_own_display_name(
    core: &Core,
    display_name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let payload = core
        .set_own_display_name(synara_core::core_api::MatrixSetOwnDisplayNameRequest {
            display_name,
        })
        .await
        .map_err(map_own_profile_core_error)?;
    Ok(payload)
}

pub(crate) async fn set_own_avatar(
    core: &Core,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let payload = core
        .set_own_avatar(synara_core::core_api::MatrixSetOwnAvatarRequest { mxc })
        .await
        .map_err(map_own_profile_core_error)?;
    Ok(payload)
}

pub(crate) async fn get_own_profile(
    core: &Core,
) -> Result<MatrixOwnProfile, MatrixAuthCommandError> {
    let payload = core
        .get_own_profile()
        .await
        .map_err(map_own_profile_core_error)?;
    Ok(payload)
}

fn map_own_profile_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-send.r-avatar-set-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix profile request is invalid.",
            diagnostic,
        ),
        _ => {
            let message = match diagnostic {
                "v-send.r-avatar-display-name-sdk-failed" => {
                    "The native Matrix display name could not be updated."
                }
                "v-send.r-avatar-display-name-read-failed" | "v-send.r-avatar-read-failed" => {
                    "The native Matrix profile could not be loaded."
                }
                _ => "The native Matrix avatar could not be updated.",
            };
            MatrixAuthCommandError::new("Unknown", message, diagnostic)
        }
    }
}
