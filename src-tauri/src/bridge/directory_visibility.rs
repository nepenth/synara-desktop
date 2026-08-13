//! Desktop bridges for room directory visibility through `Core::command`.

use synara_core::app::room_profile::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const GET_COMMAND: &str = "matrix_get_room_directory_visibility";
const SET_COMMAND: &str = "matrix_set_room_directory_visibility";

pub(crate) async fn get_room_directory_visibility(
    core: &Core,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomDirectoryVisibilityResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: GET_COMMAND.to_owned(),
            session_generation,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "sessionGeneration": session_generation,
            }),
        })
        .await
        .map_err(map_directory_visibility_core_error)?;
    parse_visibility_result(response.payload)
}

pub(crate) async fn set_room_directory_visibility(
    core: &Core,
    room_id: String,
    session_generation: u64,
    visibility: String,
) -> Result<MatrixRoomDirectoryVisibilityWriteResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: SET_COMMAND.to_owned(),
            session_generation,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "sessionGeneration": session_generation,
                "visibility": visibility,
            }),
        })
        .await
        .map_err(map_directory_visibility_core_error)?;
    parse_visibility_write_result(response.payload)
}

fn parse_visibility_result(
    payload: serde_json::Value,
) -> Result<MatrixRoomDirectoryVisibilityResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        session_generation: u64,
        visibility: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| directory_response_error())?;
    let visibility = match wire.visibility.as_str() {
        "public" => "public",
        "private" => "private",
        _ => return Err(directory_response_error()),
    };
    Ok(MatrixRoomDirectoryVisibilityResult {
        status: "ok",
        room_id: wire.room_id,
        session_generation: wire.session_generation,
        visibility,
    })
}

fn parse_visibility_write_result(
    payload: serde_json::Value,
) -> Result<MatrixRoomDirectoryVisibilityWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        session_generation: u64,
        requested_visibility: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| directory_response_error())?;
    let requested_visibility = match wire.requested_visibility.as_str() {
        "public" => "public",
        "private" => "private",
        _ => return Err(directory_response_error()),
    };
    Ok(MatrixRoomDirectoryVisibilityWriteResult {
        status: "ok",
        room_id: wire.room_id,
        session_generation: wire.session_generation,
        requested_visibility,
    })
}

fn map_directory_visibility_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-send.r-room-profile-directory-visibility-get-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => match diagnostic {
            "v-send.r-room-profile-directory-visibility-permission-denied" => {
                MatrixAuthCommandError::new(
                    "Forbidden",
                    "The native Matrix room directory visibility change is not permitted.",
                    diagnostic,
                )
            }
            _ => MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-send.r-room-profile-directory-visibility-requires-session",
            ),
        },
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "Forbidden",
            "The native Matrix room directory visibility session is stale.",
            "v-send.r-room-profile-directory-visibility-stale-generation",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-send.r-room-profile-directory-visibility-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix room directory visibility request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let message = match diagnostic {
                "v-send.r-room-profile-directory-visibility-permission-state-unavailable" => {
                    "The native Matrix room permissions are unavailable."
                }
                "v-send.r-room-profile-directory-visibility-set-sdk-failed" => {
                    "The native Matrix room directory visibility could not be updated."
                }
                _ => "The native Matrix room directory visibility could not be read.",
            };
            MatrixAuthCommandError::new("Unknown", message, diagnostic)
        }
    }
}

fn directory_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room directory visibility could not be read.",
        "v-send.r-room-profile-directory-visibility-get-sdk-failed",
    )
}
