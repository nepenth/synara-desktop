//! Desktop bridges for room leave/join through `Core::command`.

use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_leave(core: &Core, room_id: String) -> Result<(), MatrixAuthCommandError> {
    core.room_leave(synara_core::core_api::MatrixRoomLeaveRequest { room_id })
        .await
        .map_err(map_room_leave_join_core_error)?;
    Ok(())
}

pub(crate) async fn room_join(
    core: &Core,
    room_id_or_alias: String,
    via_servers: Option<Vec<String>>,
) -> Result<(), MatrixAuthCommandError> {
    core.room_join(synara_core::core_api::MatrixRoomJoinRequest {
        room_id_or_alias,
        via_servers,
    })
    .await
    .map_err(map_room_leave_join_core_error)?;
    Ok(())
}

pub(crate) async fn room_set_favorite(
    core: &Core,
    room_id: String,
    favorite: bool,
) -> Result<(), MatrixAuthCommandError> {
    core.room_set_favorite(synara_core::core_api::MatrixRoomSetFavoriteRequest {
        room_id,
        favorite,
    })
    .await
    .map_err(map_room_leave_join_core_error)?;
    Ok(())
}

fn map_room_leave_join_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
                .unwrap_or("v-rooms-room-leave-invalid-room");
            let (code, message) = match diagnostic {
                "v-rooms-room-leave-room-not-found" | "v-rooms-room-favorite-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                "v-rooms-room-join-invalid-room" | "v-rooms-room-join-invalid-via-server" => (
                    "InvalidRequest",
                    "The native Matrix room join request is invalid.",
                ),
                "v-rooms-room-favorite-invalid-room" => (
                    "InvalidRequest",
                    "The native Matrix room favorite request is invalid.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix room leave request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms-room-leave-failed");
            let message = if diagnostic.contains("join") {
                "The native Matrix room could not be joined."
            } else if diagnostic.contains("favorite") {
                "The native Matrix room favorite could not be updated."
            } else {
                "The native Matrix room could not be left."
            };
            MatrixAuthCommandError::new("Unknown", message, diagnostic)
        }
    }
}
