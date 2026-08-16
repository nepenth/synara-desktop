use super::*;

pub use synara_core::app::members::{
    NativeRoomCreatorsSnapshot, NativeRoomPowerLevelTagsSnapshot, NativeRoomPowerLevelsSnapshot,
    ROOM_CREATE_EVENT_TYPE, ROOM_POWER_LEVELS_EVENT_TYPE, ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};

#[tauri::command]
pub async fn matrix_room_members_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeRoomMembersSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_members::room_members_snapshot(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_room_power_levels_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeRoomPowerLevelsSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_members::room_power_levels_snapshot(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_room_creators_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeRoomCreatorsSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_members::room_creators_snapshot(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_room_power_level_tags_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeRoomPowerLevelTagsSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_members::room_power_level_tags_snapshot(core.inner().as_ref(), room_id)
        .await
}

pub(super) fn project_room_creators(
    event: &serde_json::Value,
) -> Result<Vec<String>, &'static str> {
    synara_core::app::members::project_room_creators(event)
}

pub(super) fn validate_power_levels_snapshot_content(
    content: &serde_json::Value,
) -> Result<(), &'static str> {
    synara_core::app::members::validate_power_levels_snapshot_content(content)
}

pub(super) fn validate_power_level_tags_snapshot_content(
    content: &serde_json::Value,
) -> Result<(), &'static str> {
    synara_core::app::members::validate_power_level_tags_snapshot_content(content)
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
        "v-rooms-members-read-power-levels-malformed"
        | "v-rooms-members-read-power-levels-too-large" => (
            "Unknown",
            "The native Matrix room power levels are unavailable.",
        ),
        "v-rooms-members-read-power-level-tags-malformed"
        | "v-rooms-members-read-power-level-tags-too-large" => (
            "Unknown",
            "The native Matrix room power-level tags are unavailable.",
        ),
        "v-rooms-members-read-creators-malformed" => (
            "Unknown",
            "The native Matrix room creators are unavailable.",
        ),
        "v-rooms-members-read-state-failed" | "v-rooms-members-read-state-malformed" => {
            ("Unknown", "The native Matrix room state is unavailable.")
        }
        _ => ("Unknown", "The native Matrix room members are unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
