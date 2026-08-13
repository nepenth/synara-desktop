//! Desktop bridges for room name/topic/avatar writes through `Core::command`.

use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::auth::product::MatrixProfileWriteResult;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn set_room_name(
    core: &Core,
    room_id: String,
    name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    dispatch_write(
        core,
        "matrix_set_room_name",
        serde_json::json!({ "roomId": room_id, "name": name }),
    )
    .await
}

pub(crate) async fn set_room_topic(
    core: &Core,
    room_id: String,
    topic: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    dispatch_write(
        core,
        "matrix_set_room_topic",
        serde_json::json!({ "roomId": room_id, "topic": topic }),
    )
    .await
}

pub(crate) async fn set_room_avatar(
    core: &Core,
    room_id: String,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    dispatch_write(
        core,
        "matrix_set_room_avatar",
        serde_json::json!({ "roomId": room_id, "mxc": mxc }),
    )
    .await
}

async fn dispatch_write(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map_err(map_room_profile_write_core_error)?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

fn map_room_profile_write_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
                .unwrap_or("d0.4-send-invalid-room-id");
            let (code, message) = match diagnostic {
                "v-send.r-room-profile-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                "v-send.r-room-profile-name-too-long" | "v-send.r-room-profile-topic-too-long" => (
                    "InvalidRequest",
                    "The native Matrix room profile request is invalid.",
                ),
                "v-send.r-avatar-invalid-mxc" => (
                    "InvalidRequest",
                    "The native Matrix avatar request is invalid.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix room profile request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-send.r-room-profile-name-sdk-failed");
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix room profile operation failed.",
                diagnostic,
            )
        }
    }
}
