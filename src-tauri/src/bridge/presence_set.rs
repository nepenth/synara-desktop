//! Desktop bridge for `matrix_presence_set` through `Core::command`.
//!
//! Core owns the live `NativePresenceOwner` after the shell attaches it. This
//! adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. Account Profile invokes `matrix_presence_set`
//! to set own presence (online / unavailable / offline).

use synara_core::app::presence::NativePresenceWriteResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn presence_set(
    core: &Core,
    state: String,
    status_msg: Option<String>,
) -> Result<NativePresenceWriteResult, MatrixAuthCommandError> {
    let response = core
        .presence_set(synara_core::core_api::MatrixPresenceSetRequest { state, status_msg })
        .await
        .map_err(map_presence_set_core_error)?;
    Ok(response)
}

fn map_presence_set_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-presence-user-owner-missing",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic_id = match error.diagnostic_id.as_deref() {
                Some("p4.7-status-msg-cap") => "p4.7-status-msg-cap",
                _ => "v-presence-state-unsupported",
            };
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix presence request is invalid.",
                diagnostic_id,
            )
        }
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "StaleSessionGeneration",
            "The native Matrix presence session changed.",
            "v-presence-stale-session-generation",
        ),
        _ => match error.diagnostic_id.as_deref() {
            Some("v-presence-set-sdk-failed") => MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix presence status could not be updated.",
                "v-presence-set-sdk-failed",
            ),
            _ => MatrixAuthCommandError::new(
                "Unknown",
                "Native Matrix presence is unavailable.",
                "v-presence-store-read-failed",
            ),
        },
    }
}
