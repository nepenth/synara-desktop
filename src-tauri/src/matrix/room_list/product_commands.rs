use super::*;

#[tauri::command]
pub async fn matrix_room_list_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_list::room_list_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_invites_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}
