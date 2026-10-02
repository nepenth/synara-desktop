use super::*;

#[cfg(test)]
const DIRECTORY_VISIBILITY_INVALID: &str = "v-send.r-room-profile-directory-visibility-invalid";
#[cfg(test)]
const DIRECTORY_VISIBILITY_REQUIRES_SESSION: &str =
    "v-send.r-room-profile-directory-visibility-requires-session";
#[cfg(test)]
const DIRECTORY_VISIBILITY_STALE_GENERATION: &str =
    "v-send.r-room-profile-directory-visibility-stale-generation";
#[cfg(test)]
const DIRECTORY_VISIBILITY_ROOM_NOT_FOUND: &str =
    "v-send.r-room-profile-directory-visibility-room-not-found";
#[cfg(test)]
const DIRECTORY_VISIBILITY_PERMISSION_DENIED: &str =
    "v-send.r-room-profile-directory-visibility-permission-denied";
#[cfg(test)]
const DIRECTORY_VISIBILITY_PERMISSION_STATE_UNAVAILABLE: &str =
    "v-send.r-room-profile-directory-visibility-permission-state-unavailable";
#[cfg(test)]
const DIRECTORY_VISIBILITY_GET_SDK_FAILED: &str =
    "v-send.r-room-profile-directory-visibility-get-sdk-failed";
#[cfg(test)]
const DIRECTORY_VISIBILITY_SET_SDK_FAILED: &str =
    "v-send.r-room-profile-directory-visibility-set-sdk-failed";

#[cfg(test)]
use matrix_sdk::ruma::OwnedRoomId;

pub use synara_core::app::room_profile::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, MatrixRoomRetentionSnapshot,
};

/// V-SEND.R-ROOM-PROFILE-JOIN-RULE — authoritative live room-scoped join-rule
/// read through the managed native Matrix SDK client.
#[tauri::command]
pub async fn matrix_room_join_rule_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomJoinRuleSnapshot, MatrixAuthCommandError> {
    crate::bridge::join_rule_snapshot::join_rule_snapshot(
        core.inner().as_ref(),
        room_id,
        session_generation,
    )
    .await
}

/// V-SEND.R-ROOM-PROFILE-JOIN-RULE — join-rule write through
/// `NativeRoomJoinRuleOwner` / `privacy_settings().update_join_rule`.
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.join_rules)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_room_set_join_rule(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    join_rule: String,
    allow_room_ids: Option<Vec<String>>,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::join_rule_snapshot::set_join_rule(
        core.inner().as_ref(),
        room_id,
        join_rule,
        allow_room_ids,
    )
    .await
}

/// V-SEND.R-ROOM-PROFILE-DIRECTORY-VISIBILITY — authoritative room-scoped
/// directory visibility read through the managed native Matrix SDK client.
#[tauri::command]
pub async fn matrix_get_room_directory_visibility(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomDirectoryVisibilityResult, MatrixAuthCommandError> {
    crate::bridge::directory_visibility::get_room_directory_visibility(
        core.inner().as_ref(),
        room_id,
        session_generation,
    )
    .await
}

/// V-SEND.R-ROOM-PROFILE-DIRECTORY-VISIBILITY — permission-checked room-scoped
/// directory visibility write through the managed native Matrix SDK client.
/// The returned value acknowledges the PUT only; the frontend must perform a
/// fresh `matrix_get_room_directory_visibility` before displaying success.
#[tauri::command]
pub async fn matrix_set_room_directory_visibility(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    session_generation: u64,
    visibility: String,
) -> Result<MatrixRoomDirectoryVisibilityWriteResult, MatrixAuthCommandError> {
    crate::bridge::directory_visibility::set_room_directory_visibility(
        core.inner().as_ref(),
        room_id,
        session_generation,
        visibility,
    )
    .await
}

/// R-ROOM-PROFILE — sole native owner for a room's display name write.
/// Empty/whitespace-only input clears the name (sends an empty `m.room.name`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.name)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_name(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    name: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_profile_writes::set_room_name(core.inner().as_ref(), room_id, name).await
}

/// R-ROOM-PROFILE — sole native owner for a room's topic write.
/// Empty/whitespace-only input clears the topic (sends an empty `m.room.topic`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.topic)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_topic(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    topic: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_profile_writes::set_room_topic(core.inner().as_ref(), room_id, topic).await
}

/// R-ROOM-PROFILE — sole native owner for a room's avatar URL write.
/// Empty string removes the avatar (`room.remove_avatar()`). The `mxc` must be
/// a valid `mxc://` URI (typically produced by `matrix_upload_media`).
/// Fail-closed: when a native session is live this command is the only path;
/// the JS `mx.sendStateEvent(m.room.avatar)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_avatar(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    mxc: String,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_profile_writes::set_room_avatar(core.inner().as_ref(), room_id, mxc).await
}

/// Read-only MSC1763 retention copy. Unknown homeserver config is not forever.
#[tauri::command]
pub async fn matrix_room_retention(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomRetentionSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_retention::room_retention(
        core.inner().as_ref(),
        room_id,
        session_generation,
    )
    .await
}

/// Leftover encryptable state writes (canonical alias, ACL, developer-tools).
/// Fail-closed: native sessions must not JS-plaintext PUT these types.
#[tauri::command]
pub async fn matrix_send_state_event(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_type: String,
    state_key: Option<String>,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_profile_writes::send_state_event(
        core.inner().as_ref(),
        room_id,
        event_type,
        state_key.unwrap_or_default(),
        content,
    )
    .await
}

/// Enable room encryption and/or opt an already-E2EE room into MSC4362.
/// Native send of `m.room.encryption` (excluded type). Setting-off and call
/// rooms never write the flag.
#[tauri::command]
pub async fn matrix_enable_room_encrypted_state(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    encrypt_state_events: Option<bool>,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_profile_writes::enable_room_encrypted_state(
        core.inner().as_ref(),
        room_id,
        encrypt_state_events.unwrap_or(false),
    )
    .await
}
#[cfg(test)]
pub(super) fn parse_room_directory_visibility_room_id(
    room_id: &str,
) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .parse()
        .map_err(|_| map_room_directory_visibility_error(DIRECTORY_VISIBILITY_INVALID))
}
#[cfg(test)]
pub(super) fn parse_room_directory_visibility(
    visibility: &str,
) -> Result<(Visibility, &'static str), MatrixAuthCommandError> {
    match visibility {
        "public" => Ok((Visibility::Public, "public")),
        "private" => Ok((Visibility::Private, "private")),
        _ => Err(map_room_directory_visibility_error(
            DIRECTORY_VISIBILITY_INVALID,
        )),
    }
}
#[cfg(test)]
pub(super) fn map_room_directory_visibility_error(
    diagnostic_id: &'static str,
) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        DIRECTORY_VISIBILITY_INVALID => (
            "InvalidRequest",
            "The native Matrix room directory visibility request is invalid.",
        ),
        DIRECTORY_VISIBILITY_REQUIRES_SESSION => {
            ("Forbidden", "No native Matrix session is active.")
        }
        DIRECTORY_VISIBILITY_STALE_GENERATION => (
            "Forbidden",
            "The native Matrix room directory visibility session is stale.",
        ),
        DIRECTORY_VISIBILITY_ROOM_NOT_FOUND => {
            ("NotFound", "The native Matrix room is not available.")
        }
        DIRECTORY_VISIBILITY_PERMISSION_DENIED => (
            "Forbidden",
            "The native Matrix room directory visibility change is not permitted.",
        ),
        DIRECTORY_VISIBILITY_PERMISSION_STATE_UNAVAILABLE => (
            "Unknown",
            "The native Matrix room permissions are unavailable.",
        ),
        DIRECTORY_VISIBILITY_GET_SDK_FAILED => (
            "Unknown",
            "The native Matrix room directory visibility could not be read.",
        ),
        DIRECTORY_VISIBILITY_SET_SDK_FAILED => (
            "Unknown",
            "The native Matrix room directory visibility could not be updated.",
        ),
        _ => (
            "Unknown",
            "The native Matrix room directory visibility operation failed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
