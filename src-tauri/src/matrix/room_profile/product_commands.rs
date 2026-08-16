use super::*;
use matrix_sdk::ruma::OwnedRoomId;

const DIRECTORY_VISIBILITY_INVALID: &str = "v-send.r-room-profile-directory-visibility-invalid";
const DIRECTORY_VISIBILITY_REQUIRES_SESSION: &str =
    "v-send.r-room-profile-directory-visibility-requires-session";
const DIRECTORY_VISIBILITY_STALE_GENERATION: &str =
    "v-send.r-room-profile-directory-visibility-stale-generation";
const DIRECTORY_VISIBILITY_ROOM_NOT_FOUND: &str =
    "v-send.r-room-profile-directory-visibility-room-not-found";
const DIRECTORY_VISIBILITY_PERMISSION_DENIED: &str =
    "v-send.r-room-profile-directory-visibility-permission-denied";
const DIRECTORY_VISIBILITY_PERMISSION_STATE_UNAVAILABLE: &str =
    "v-send.r-room-profile-directory-visibility-permission-state-unavailable";
const DIRECTORY_VISIBILITY_GET_SDK_FAILED: &str =
    "v-send.r-room-profile-directory-visibility-get-sdk-failed";
const DIRECTORY_VISIBILITY_SET_SDK_FAILED: &str =
    "v-send.r-room-profile-directory-visibility-set-sdk-failed";

const JOIN_RULE_INVALID: &str = "v-send.r-room-profile-join-rule-invalid";
const JOIN_RULE_REQUIRES_SESSION: &str = "v-send.r-room-profile-join-rule-requires-session";
const JOIN_RULE_STALE_GENERATION: &str = "v-send.r-room-profile-join-rule-stale-generation";
const JOIN_RULE_ROOM_NOT_FOUND: &str = "v-send.r-room-profile-join-rule-room-not-found";
const JOIN_RULE_ROOM_STATE_UNAVAILABLE: &str =
    "v-send.r-room-profile-join-rule-room-state-unavailable";
const JOIN_RULE_READ_SDK_FAILED: &str = "v-send.r-room-profile-join-rule-read-sdk-failed";
const JOIN_RULE_DESERIALIZE_FAILED: &str = "v-send.r-room-profile-join-rule-deserialize-failed";
const JOIN_RULE_UNSUPPORTED: &str = "v-send.r-room-profile-join-rule-unsupported";

pub use synara_core::app::room_profile::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot,
};

/// V-SEND.R-ROOM-PROFILE-JOIN-RULE — authoritative live room-scoped join-rule
/// read through the managed native Matrix SDK client. This is intentionally a
/// read-only gate owner; the Join Rules writer remains a separate residual.
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

pub(super) fn parse_room_directory_visibility_room_id(
    room_id: &str,
) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .parse()
        .map_err(|_| map_room_directory_visibility_error(DIRECTORY_VISIBILITY_INVALID))
}

pub(super) fn parse_room_join_rule_room_id(
    room_id: &str,
) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    if room_id.is_empty()
        || room_id.len() > 512
        || room_id.trim() != room_id
        || room_id.chars().any(char::is_whitespace)
        || !room_id.starts_with('!')
    {
        return Err(map_room_join_rule_error(JOIN_RULE_INVALID));
    }
    room_id
        .parse()
        .map_err(|_| map_room_join_rule_error(JOIN_RULE_INVALID))
}

fn require_room_join_rule_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| map_room_join_rule_error(JOIN_RULE_REQUIRES_SESSION))
}

fn require_room_join_rule_generation(
    requested: u64,
    live: u64,
) -> Result<(), MatrixAuthCommandError> {
    if requested == 0 || requested != live {
        return Err(map_room_join_rule_error(JOIN_RULE_STALE_GENERATION));
    }
    Ok(())
}

pub(super) fn map_room_join_rule_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        JOIN_RULE_INVALID => (
            "InvalidRequest",
            "The native Matrix room join-rule request is invalid.",
        ),
        JOIN_RULE_REQUIRES_SESSION => ("Forbidden", "No native Matrix session is active."),
        JOIN_RULE_STALE_GENERATION => (
            "Forbidden",
            "The native Matrix room join-rule session is stale.",
        ),
        JOIN_RULE_ROOM_NOT_FOUND => ("NotFound", "The native Matrix room is not available."),
        JOIN_RULE_ROOM_STATE_UNAVAILABLE
        | JOIN_RULE_DESERIALIZE_FAILED
        | JOIN_RULE_UNSUPPORTED
        | JOIN_RULE_READ_SDK_FAILED => (
            "Unknown",
            "The native Matrix room join rule is unavailable.",
        ),
        _ => (
            "Unknown",
            "The native Matrix room join-rule operation failed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

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
