//! Typed SharedCore operations for user room folders (`in.synara.room_folders`).
//!
//! Sections in the iOS room list come from `RoomListSnapshotDto.folders`
//! (already resolved with the section rule) and `RoomListRoomDto.folder_id`;
//! `room_section_placement` is the same rule for one row. Mutations return the
//! folders as written, and the room list re-emits before the sync echo.

use super::*;
use crate::app::account_data::{
    NativeRoomFolderCreateResult, NativeRoomFoldersSnapshot, RoomFoldersError,
};
use crate::app::room_list::RoomFolderPresentation;

const ROOM_FOLDERS_FAILED_CODE: &str = "room-folders-failed";
const ROOM_FOLDERS_FAILED_DESCRIPTION: &str = "The room folders request could not be completed.";
const ROOM_FOLDERS_NO_SESSION_DESCRIPTION: &str = "No room folders session is available.";
const ROOM_FOLDERS_INVALID_DESCRIPTION: &str = "The room folders request is invalid.";
const ROOM_FOLDERS_UNAVAILABLE_DESCRIPTION: &str = "Room folders are unavailable right now.";

/// One stored folder. `room_ids` is every filed room in filing order,
/// favorites and rooms no longer listed included.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomFolderDto {
    pub id: String,
    pub name: String,
    pub room_ids: Vec<String>,
}

/// The stored folders in display order. `read_only` means a newer client
/// wrote the data: show it, but mutations fail with
/// `v-room-folders.unsupported-version`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomFoldersSnapshotDto {
    pub session_generation: u64,
    pub version: u32,
    pub read_only: bool,
    pub folders: Vec<RoomFolderDto>,
}

/// A created folder's id and the folders as written.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomFolderCreateDto {
    pub folder_id: String,
    pub snapshot: RoomFoldersSnapshotDto,
}

/// One folder section of the room list, resolved by Core: the rooms that
/// show in it (favorites, DMs, spaces and unlisted rooms excluded) in filing
/// order and in both section orders, plus the unread rollup.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomFolderSectionDto {
    pub id: String,
    pub name: String,
    pub room_ids: Vec<String>,
    /// Newest activity first, low priority last (`order_room_ids` Recent).
    pub recent_order: Vec<String>,
    /// By name, low priority last (`order_room_ids` Name).
    pub name_order: Vec<String>,
    pub unread_highlight: u32,
    pub unread_total: u32,
    pub unread_room_ids: Vec<String>,
}

impl From<RoomFolderPresentation> for RoomFolderSectionDto {
    fn from(folder: RoomFolderPresentation) -> Self {
        Self {
            id: folder.id,
            name: folder.name,
            room_ids: folder.room_ids,
            recent_order: folder.recent_order,
            name_order: folder.name_order,
            unread_highlight: folder.unread_highlight,
            unread_total: folder.unread_total,
            unread_room_ids: folder.unread_room_ids,
        }
    }
}

/// Static fail-closed room-folders error. `code` is a source constant: a
/// `p2-room-folders-*-no-session` id, a `v-room-folders.*` diagnostic id
/// (for example `v-room-folders.room-is-direct`), or `room-folders-failed`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomFoldersCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomFoldersCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomFoldersCommandError {}

fn room_folders_failed(code: &str, description: &'static str) -> RoomFoldersCommandError {
    RoomFoldersCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_folders_core_error(error: MatrixIpcError) -> RoomFoldersCommandError {
    let Some(code) = error.diagnostic_id.as_deref() else {
        return room_folders_failed(ROOM_FOLDERS_FAILED_CODE, ROOM_FOLDERS_FAILED_DESCRIPTION);
    };
    if code.starts_with("p2-room-folders-") && code.ends_with("-no-session") {
        return room_folders_failed(code, ROOM_FOLDERS_NO_SESSION_DESCRIPTION);
    }
    match RoomFoldersError::from_diagnostic_id(code) {
        Some(error) if error.is_invalid_request() => {
            room_folders_failed(code, ROOM_FOLDERS_INVALID_DESCRIPTION)
        }
        Some(_) => room_folders_failed(code, ROOM_FOLDERS_UNAVAILABLE_DESCRIPTION),
        None => room_folders_failed(ROOM_FOLDERS_FAILED_CODE, ROOM_FOLDERS_FAILED_DESCRIPTION),
    }
}

pub(super) fn room_folders_snapshot_dto(
    snapshot: NativeRoomFoldersSnapshot,
) -> RoomFoldersSnapshotDto {
    RoomFoldersSnapshotDto {
        session_generation: snapshot.session_generation,
        version: snapshot.version,
        read_only: snapshot.read_only,
        folders: snapshot
            .folders
            .into_iter()
            .map(|folder| RoomFolderDto {
                id: folder.id,
                name: folder.name,
                room_ids: folder.room_ids,
            })
            .collect(),
    }
}

fn room_folder_create_dto(result: NativeRoomFolderCreateResult) -> RoomFolderCreateDto {
    RoomFolderCreateDto {
        folder_id: result.folder_id,
        snapshot: room_folders_snapshot_dto(result.snapshot),
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// The stored folders (pending local writes included).
    pub async fn room_folders_snapshot(
        &self,
    ) -> Result<RoomFoldersSnapshotDto, RoomFoldersCommandError> {
        self.core
            .room_folders_snapshot()
            .await
            .map(room_folders_snapshot_dto)
            .map_err(map_room_folders_core_error)
    }

    /// Append a folder; with `room_id`, file that room in it in one write.
    pub async fn room_folders_create(
        &self,
        name: String,
        room_id: Option<String>,
    ) -> Result<RoomFolderCreateDto, RoomFoldersCommandError> {
        self.core
            .room_folders_create(crate::core_api::MatrixRoomFoldersCreateRequest { name, room_id })
            .await
            .map(room_folder_create_dto)
            .map_err(map_room_folders_core_error)
    }

    pub async fn room_folders_rename(
        &self,
        folder_id: String,
        name: String,
    ) -> Result<RoomFoldersSnapshotDto, RoomFoldersCommandError> {
        self.core
            .room_folders_rename(crate::core_api::MatrixRoomFoldersRenameRequest {
                folder_id,
                name,
            })
            .await
            .map(room_folders_snapshot_dto)
            .map_err(map_room_folders_core_error)
    }

    /// Delete a folder; its rooms become unfiled.
    pub async fn room_folders_delete(
        &self,
        folder_id: String,
    ) -> Result<RoomFoldersSnapshotDto, RoomFoldersCommandError> {
        self.core
            .room_folders_delete(crate::core_api::MatrixRoomFoldersDeleteRequest { folder_id })
            .await
            .map(room_folders_snapshot_dto)
            .map_err(map_room_folders_core_error)
    }

    /// Move a folder to `to_index` in display order (past the end = last).
    pub async fn room_folders_move(
        &self,
        folder_id: String,
        to_index: u32,
    ) -> Result<RoomFoldersSnapshotDto, RoomFoldersCommandError> {
        self.core
            .room_folders_move(crate::core_api::MatrixRoomFoldersMoveRequest {
                folder_id,
                to_index,
            })
            .await
            .map(room_folders_snapshot_dto)
            .map_err(map_room_folders_core_error)
    }

    /// File a room (`folder_id`) or unfile it (`None`). Filing rejects DMs,
    /// spaces and rooms the user has not joined.
    pub async fn room_folders_set_room(
        &self,
        room_id: String,
        folder_id: Option<String>,
    ) -> Result<RoomFoldersSnapshotDto, RoomFoldersCommandError> {
        self.core
            .room_folders_set_room(crate::core_api::MatrixRoomFoldersSetRoomRequest {
                room_id,
                folder_id,
            })
            .await
            .map(room_folders_snapshot_dto)
            .map_err(map_room_folders_core_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MatrixIpcErrorCategory;

    fn code(error: RoomFoldersCommandError) -> String {
        match error {
            RoomFoldersCommandError::Failed { code, .. } => code,
        }
    }

    #[test]
    fn core_errors_map_to_stable_codes() {
        let error = |category, id: &str| MatrixIpcError::new(category).with_diagnostic(id);
        assert_eq!(
            code(map_room_folders_core_error(error(
                MatrixIpcErrorCategory::Forbidden,
                "p2-room-folders-create-no-session"
            ))),
            "p2-room-folders-create-no-session"
        );
        assert_eq!(
            code(map_room_folders_core_error(error(
                MatrixIpcErrorCategory::SdkInvariant,
                "v-room-folders.room-is-direct"
            ))),
            "v-room-folders.room-is-direct"
        );
        assert_eq!(
            code(map_room_folders_core_error(error(
                MatrixIpcErrorCategory::Unknown,
                "something-else"
            ))),
            ROOM_FOLDERS_FAILED_CODE
        );
        assert_eq!(
            code(map_room_folders_core_error(MatrixIpcError::new(
                MatrixIpcErrorCategory::Unknown
            ))),
            ROOM_FOLDERS_FAILED_CODE
        );
    }
}
