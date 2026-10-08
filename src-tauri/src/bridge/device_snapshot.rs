//! Desktop bridge for `matrix_device_snapshot` through `Core::command`.
//!
//! Core owns the live `NativeDeviceOwner` after the shell attaches it. This
//! adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_device_snapshot`.

use synara_core::app::devices::NativeDeviceSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn device_snapshot(
    core: &Core,
) -> Result<NativeDeviceSnapshot, MatrixAuthCommandError> {
    let response = core
        .device_snapshot()
        .await
        .map_err(map_device_snapshot_core_error)?;
    Ok(response)
}

fn map_device_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.7-device-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix device management is unavailable.",
            "v-crypto.7-device-snapshot-server-failed",
        ),
    }
}
