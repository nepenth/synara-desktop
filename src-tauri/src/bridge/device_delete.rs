//! Desktop bridge for device delete start/cancel through `Core::command`.
//!
//! Password UIAA stays in the desktop product command so the password never
//! crosses this envelope. React names are unchanged.

use synara_core::app::devices::NativeDeviceDeleteResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const DEVICE_DELETE_START_COMMAND: &str = "matrix_device_delete_start";
const DEVICE_DELETE_CANCEL_COMMAND: &str = "matrix_device_delete_cancel";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn device_delete_start(
    core: &Core,
    device_ids: Vec<String>,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: DEVICE_DELETE_START_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "deviceIds": device_ids }),
        })
        .await
        .map_err(map_device_delete_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| device_delete_response_error())
}

pub(crate) async fn device_delete_cancel(
    core: &Core,
    operation_id: u64,
    session_generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: DEVICE_DELETE_CANCEL_COMMAND.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "operationId": operation_id,
            "sessionGeneration": session_generation,
        }),
    })
    .await
    .map_err(map_device_delete_core_error)?;
    Ok(())
}

fn map_device_delete_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => match error.diagnostic_id.as_deref() {
            Some("v-crypto.7-device-delete-auth-unsupported") => MatrixAuthCommandError::new(
                "Forbidden",
                "The homeserver requires an unsupported authentication step for device logout.",
                "v-crypto.7-device-delete-auth-unsupported",
            ),
            _ => MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-crypto.7-device-requires-session",
            ),
        },
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "StaleSessionGeneration",
            "The native Matrix session changed during device logout.",
            "v-crypto.7-device-delete-stale-generation",
        ),
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-crypto.7-device-delete-selection-empty") => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix device request is invalid.",
                "v-crypto.7-device-delete-selection-empty",
            ),
            Some("v-crypto.7-device-delete-not-pending") => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix device request is invalid.",
                "v-crypto.7-device-delete-not-pending",
            ),
            Some("v-crypto.7-device-delete-operation-mismatch") => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix device request is invalid.",
                "v-crypto.7-device-delete-operation-mismatch",
            ),
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix device request is invalid.",
                "v-crypto.7-device-delete-selection-invalid",
            ),
        },
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix device management is unavailable.",
            "v-crypto.7-device-delete-start-failed",
        ),
    }
}

fn device_delete_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix device management is unavailable.",
        "v-crypto.7-device-delete-start-failed",
    )
}
