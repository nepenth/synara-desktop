//! Desktop bridges for room directory visibility through `Core::command`.

use synara_core::app::room_profile::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn get_room_directory_visibility(
    core: &Core,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomDirectoryVisibilityResult, MatrixAuthCommandError> {
    let response = core
        .get_room_directory_visibility(
            synara_core::core_api::MatrixGetRoomDirectoryVisibilityRequest {
                room_id,
                session_generation,
            },
        )
        .await
        .map_err(map_directory_visibility_core_error)?;
    Ok(response)
}

pub(crate) async fn set_room_directory_visibility(
    core: &Core,
    room_id: String,
    session_generation: u64,
    visibility: String,
) -> Result<MatrixRoomDirectoryVisibilityWriteResult, MatrixAuthCommandError> {
    let response = core
        .set_room_directory_visibility(
            synara_core::core_api::MatrixSetRoomDirectoryVisibilityRequest {
                room_id,
                session_generation,
                visibility,
            },
        )
        .await
        .map_err(map_directory_visibility_core_error)?;
    Ok(response)
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
