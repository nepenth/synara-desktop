//! Desktop bridge for `matrix_device_rename` through `Core::command`.
//!
//! Core owns the live `NativeDeviceOwner` after the shell attaches it. This
//! adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_device_rename`.

use synara_core::app::devices::NativeDeviceSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn device_rename(
    core: &Core,
    device_id: String,
    display_name: String,
) -> Result<NativeDeviceSnapshot, MatrixAuthCommandError> {
    let response = core
        .device_rename(synara_core::core_api::MatrixDeviceRenameRequest {
            device_id,
            display_name,
        })
        .await
        .map_err(map_device_rename_core_error)?;
    Ok(response)
}

fn map_device_rename_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.7-device-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix device request is invalid.",
            "v-crypto.7-device-rename-empty",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix device management is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-crypto.7-device-rename-failed"),
        ),
    }
}
