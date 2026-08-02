use super::*;

#[tauri::command]
pub async fn matrix_room_members_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeRoomMembersSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let room_id = parse_room_members_room_id(&room_id).map_err(map_room_members_error)?;
    let room = active
        .client
        .get_room(&room_id)
        .ok_or_else(|| map_room_members_error("v-rooms-members-read-room-not-found"))?;
    let is_direct = room.is_direct().await.unwrap_or(false);
    let current_user = active.client.user_id();
    let sdk_members = room
        .members(RoomMemberships::empty())
        .await
        .map_err(|_| map_room_members_error("v-rooms-members-read-members-failed"))?;
    let is_two_party_direct = is_direct && sdk_members.len() == 2;

    let mut members = sdk_members
        .iter()
        .map(|member| project_room_member(&room_id, member, is_two_party_direct, current_user))
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_room_members_error)?;
    members.sort_by(|left, right| left.user_id.cmp(&right.user_id));

    Ok(NativeRoomMembersSnapshot {
        session_generation: active.sync.session_generation(),
        room_id: room_id.to_string(),
        members,
    })
}

pub(super) fn map_room_members_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms-members-read-invalid-room" => (
            "InvalidRequest",
            "The native Matrix room members request is invalid.",
        ),
        "v-rooms-members-read-room-not-found" => (
            "NotFound",
            "The native Matrix room members are unavailable.",
        ),
        _ => ("Unknown", "The native Matrix room members are unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
