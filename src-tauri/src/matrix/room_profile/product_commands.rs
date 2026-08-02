use super::*;

/// R-ROOM-PROFILE — sole native owner for a room's display name write.
/// Empty/whitespace-only input clears the name (sends an empty `m.room.name`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.name)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_name(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let name = parse_room_name(&name)?;
    let room_id = parse_send_room_id(&room_id)?;
    let room = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.r-room-profile-room-not-found",
            )
        })?
    };
    room.set_name(name).await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room name could not be updated.",
            "v-send.r-room-profile-name-sdk-failed",
        )
    })?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

/// R-ROOM-PROFILE — sole native owner for a room's topic write.
/// Empty/whitespace-only input clears the topic (sends an empty `m.room.topic`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.topic)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_topic(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    topic: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let topic = parse_room_topic(&topic)?;
    let room_id = parse_send_room_id(&room_id)?;
    let room = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.r-room-profile-room-not-found",
            )
        })?
    };
    room.set_room_topic(&topic).await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room topic could not be updated.",
            "v-send.r-room-profile-topic-sdk-failed",
        )
    })?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

/// R-ROOM-PROFILE — sole native owner for a room's avatar URL write.
/// Empty string removes the avatar (`room.remove_avatar()`). The `mxc` must be
/// a valid `mxc://` URI (typically produced by `matrix_upload_media`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.avatar)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_avatar(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let mxc = parse_avatar_mxc(&mxc)?;
    let room_id = parse_send_room_id(&room_id)?;
    let room = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.r-room-profile-room-not-found",
            )
        })?
    };
    match mxc {
        Some(url) => {
            room.set_avatar_url(&url, None).await.map_err(|_| {
                MatrixAuthCommandError::new(
                    "Unknown",
                    "The native Matrix room avatar could not be updated.",
                    "v-send.r-room-profile-avatar-set-sdk-failed",
                )
            })?;
        }
        None => {
            room.remove_avatar().await.map_err(|_| {
                MatrixAuthCommandError::new(
                    "Unknown",
                    "The native Matrix room avatar could not be removed.",
                    "v-send.r-room-profile-avatar-remove-sdk-failed",
                )
            })?;
        }
    }
    Ok(MatrixProfileWriteResult { status: "ok" })
}

/// Parse and validate a room name. Empty/whitespace-only input clears the
/// `m.room.name` state. Non-empty names are trimmed and capped.
pub(super) fn parse_room_name(name: &str) -> Result<String, MatrixAuthCommandError> {
    let trimmed = name.trim();
    if trimmed.chars().count() > 255 {
        return Err(map_room_profile_error(
            "v-send.r-room-profile-name-too-long",
        ));
    }
    Ok(trimmed.to_owned())
}

/// Parse and validate a room topic. Empty/whitespace-only input clears the
/// `m.room.topic` state. Non-empty topics are trimmed and capped.
pub(super) fn parse_room_topic(topic: &str) -> Result<String, MatrixAuthCommandError> {
    let trimmed = topic.trim();
    if trimmed.chars().count() > 2_048 {
        return Err(map_room_profile_error(
            "v-send.r-room-profile-topic-too-long",
        ));
    }
    Ok(trimmed.to_owned())
}

pub(super) fn map_room_profile_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-send.r-room-profile-name-too-long" | "v-send.r-room-profile-topic-too-long" => {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix room profile request is invalid.",
                diagnostic_id,
            )
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room profile operation failed.",
            diagnostic_id,
        ),
    }
}
