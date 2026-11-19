//! Desktop bridges for user room folders (`in.synara.room_folders`) through
//! the typed Core API.

use synara_core::app::account_data::{
    NativeRoomFolderCreateResult, NativeRoomFoldersSnapshot, RoomFoldersError,
};
use synara_core::core_api::{
    MatrixRoomFoldersCreateRequest, MatrixRoomFoldersDeleteRequest, MatrixRoomFoldersMoveRequest,
    MatrixRoomFoldersRenameRequest, MatrixRoomFoldersSetRoomRequest,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_folders_snapshot(
    core: &Core,
) -> Result<NativeRoomFoldersSnapshot, MatrixAuthCommandError> {
    core.room_folders_snapshot()
        .await
        .map_err(map_room_folders_core_error)
}

pub(crate) async fn room_folders_create(
    core: &Core,
    name: String,
    room_id: Option<String>,
) -> Result<NativeRoomFolderCreateResult, MatrixAuthCommandError> {
    core.room_folders_create(MatrixRoomFoldersCreateRequest { name, room_id })
        .await
        .map_err(map_room_folders_core_error)
}

pub(crate) async fn room_folders_rename(
    core: &Core,
    folder_id: String,
    name: String,
) -> Result<NativeRoomFoldersSnapshot, MatrixAuthCommandError> {
    core.room_folders_rename(MatrixRoomFoldersRenameRequest { folder_id, name })
        .await
        .map_err(map_room_folders_core_error)
}

pub(crate) async fn room_folders_delete(
    core: &Core,
    folder_id: String,
) -> Result<NativeRoomFoldersSnapshot, MatrixAuthCommandError> {
    core.room_folders_delete(MatrixRoomFoldersDeleteRequest { folder_id })
        .await
        .map_err(map_room_folders_core_error)
}

pub(crate) async fn room_folders_move(
    core: &Core,
    folder_id: String,
    to_index: u32,
) -> Result<NativeRoomFoldersSnapshot, MatrixAuthCommandError> {
    core.room_folders_move(MatrixRoomFoldersMoveRequest {
        folder_id,
        to_index,
    })
    .await
    .map_err(map_room_folders_core_error)
}

pub(crate) async fn room_folders_set_room(
    core: &Core,
    room_id: String,
    folder_id: Option<String>,
) -> Result<NativeRoomFoldersSnapshot, MatrixAuthCommandError> {
    core.room_folders_set_room(MatrixRoomFoldersSetRoomRequest { room_id, folder_id })
        .await
        .map_err(map_room_folders_core_error)
}

/// `code` is `Forbidden` (no session), `InvalidRequest` (the
/// `v-room-folders.*` diagnostic says why, for example
/// `v-room-folders.room-is-direct`) or `Unknown` (I/O).
fn map_room_folders_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error.diagnostic_id.as_deref().unwrap_or("");
    let known = RoomFoldersError::from_diagnostic_id(diagnostic);
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The room folders request is invalid.",
            known.map_or(
                "v-room-folders.invalid-request",
                RoomFoldersError::diagnostic_id,
            ),
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Room folders are unavailable right now.",
            known.map_or(
                RoomFoldersError::FetchFailed.diagnostic_id(),
                RoomFoldersError::diagnostic_id,
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_keep_their_folder_diagnostic() {
        let invalid = map_room_folders_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-room-folders.room-is-space"),
        );
        let json = serde_json::to_value(&invalid).unwrap();
        assert_eq!(json["code"], "InvalidRequest");
        assert_eq!(json["diagnosticId"], "v-room-folders.room-is-space");
        let forbidden = map_room_folders_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-folders-snapshot-no-session"),
        );
        assert_eq!(
            serde_json::to_value(&forbidden).unwrap()["code"],
            "Forbidden"
        );
    }
}
