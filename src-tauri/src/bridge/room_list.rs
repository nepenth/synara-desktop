//! Desktop bridge for `matrix_room_list_snapshot` and
//! `matrix_room_list_snapshot_since` through `Core`.

use synara_core::app::room_list::{NativeRoomListSnapshot, RoomListSnapshotReply};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_list_snapshot(
    core: &Core,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    let response = core
        .room_list_snapshot()
        .await
        .map_err(map_room_list_core_error)?;
    Ok(response)
}

pub(crate) async fn room_list_snapshot_since(
    core: &Core,
    known_revision: Option<u64>,
) -> Result<RoomListSnapshotReply, MatrixAuthCommandError> {
    core.room_list_snapshot_since(known_revision)
        .await
        .map_err(map_room_list_core_error)
}

fn map_room_list_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.2-room-list-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room list is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("d0.2-room-list-open-failed"),
        ),
    }
}
