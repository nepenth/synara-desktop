//! Desktop bridges for own display-name / avatar reads and writes through `Core::command`.

use synara_core::app::user_profile::{MatrixOwnProfile, MatrixProfileWriteResult};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn set_own_display_name(
    core: &Core,
    display_name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_set_own_display_name",
        serde_json::json!({ "displayName": display_name }),
    )
    .await?;
    parse_write_result(
        payload,
        "The native Matrix display name could not be updated.",
        "v-send.r-avatar-display-name-sdk-failed",
    )
}

pub(crate) async fn set_own_avatar(
    core: &Core,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_set_own_avatar",
        serde_json::json!({ "mxc": mxc }),
    )
    .await?;
    parse_write_result(
        payload,
        "The native Matrix avatar could not be updated.",
        "v-send.r-avatar-set-sdk-failed",
    )
}

pub(crate) async fn get_own_profile(
    core: &Core,
) -> Result<MatrixOwnProfile, MatrixAuthCommandError> {
    let payload = dispatch(core, "matrix_get_own_profile", serde_json::Value::Null).await?;
    serde_json::from_value(payload).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix profile could not be loaded.",
            "v-send.r-avatar-read-failed",
        )
    })
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
    .map_err(map_own_profile_core_error)
}

fn parse_write_result(
    payload: serde_json::Value,
    unknown_message: &'static str,
    fallback: &'static str,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    struct Wire {
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload)
        .map_err(|_| MatrixAuthCommandError::new("Unknown", unknown_message, fallback))?;
    if wire.status != "ok" {
        return Err(MatrixAuthCommandError::new(
            "Unknown",
            unknown_message,
            fallback,
        ));
    }
    Ok(MatrixProfileWriteResult { status: "ok" })
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
