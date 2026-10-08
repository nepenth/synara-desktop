//! Desktop bridges for `m.direct` through `Core::command`.

use synara_core::app::account_data::{NativeMDirectMutationResult, NativeMDirectSnapshot};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn mdirect_snapshot(
    core: &Core,
) -> Result<NativeMDirectSnapshot, MatrixAuthCommandError> {
    let response = core
        .mdirect_snapshot()
        .await
        .map_err(map_mdirect_core_error)?;
    Ok(response)
}

pub(crate) async fn mdirect_add(
    core: &Core,
    room_id: String,
    user_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let response = core
        .mdirect_add(synara_core::core_api::MatrixMDirectAddRequest { room_id, user_id })
        .await
        .map_err(map_mdirect_core_error)?;
    Ok(response)
}

pub(crate) async fn mdirect_remove(
    core: &Core,
    room_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let response = core
        .mdirect_remove(synara_core::core_api::MatrixMDirectRemoveRequest { room_id })
        .await
        .map_err(map_mdirect_core_error)?;
    Ok(response)
}

fn map_mdirect_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.5-mdirect-invalid-room");
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix direct-room request is invalid.",
                diagnostic,
            )
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.5-mdirect-fetch-failed");
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix direct-room map is unavailable.",
                diagnostic,
            )
        }
    }
}
