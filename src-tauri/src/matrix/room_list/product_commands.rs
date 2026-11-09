use super::*;

#[tauri::command]
pub async fn matrix_room_list_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_list::room_list_snapshot(core.inner().as_ref()).await
}

/// `knownRevision` (the revision of the snapshot the renderer holds) turns
/// the read into a cheap probe: `{ sessionGeneration, revision }` while the
/// room list is still at that revision, else `{ revision, snapshot }`.
#[tauri::command]
pub async fn matrix_room_list_snapshot_since(
    core: State<'_, Arc<synara_core::Core>>,
    known_revision: Option<u64>,
) -> Result<synara_core::app::room_list::RoomListSnapshotReply, MatrixAuthCommandError> {
    crate::bridge::room_list::room_list_snapshot_since(core.inner().as_ref(), known_revision).await
}

#[tauri::command]
pub async fn matrix_invites_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    crate::bridge::invites_snapshot::invites_snapshot(core.inner().as_ref()).await
}
