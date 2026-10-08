//! Desktop bridge for `matrix_room_key_transfer_status` through `Core::command`.

use synara_core::app::room_keys::NativeRoomKeyTransferStatus;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_key_transfer_status(
    core: &Core,
) -> Result<NativeRoomKeyTransferStatus, MatrixAuthCommandError> {
    let response = core
        .room_key_transfer_status()
        .await
        .map_err(map_room_key_status_core_error)?;
    Ok(response)
}

fn map_room_key_status_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.5-room-keys-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room-key transfer status is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-crypto.5-status-unavailable"),
        ),
    }
}
