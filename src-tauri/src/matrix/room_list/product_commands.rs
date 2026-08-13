use super::*;

#[tauri::command]
pub async fn matrix_room_list_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_list::room_list_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_invites_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    crate::bridge::invites_snapshot::invites_snapshot(core.inner().as_ref()).await
}
