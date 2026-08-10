use super::*;

#[tauri::command]
pub async fn matrix_room_list_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = session.as_ref().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.2-room-list-requires-session",
        )
    })?;
    snapshot_from_sync_owner(&active.sync)
        .await
        .map_err(map_room_list_error)
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

pub(super) fn map_room_list_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room list is unavailable.",
        diagnostic_id,
    )
}
