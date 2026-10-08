//! Desktop bridges for members/power-level snapshots through `Core::command`.

use synara_core::app::members::{
    NativeRoomCreatorsSnapshot, NativeRoomMembersSnapshot, NativeRoomPowerLevelTagsSnapshot,
    NativeRoomPowerLevelsSnapshot,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_members_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomMembersSnapshot, MatrixAuthCommandError> {
    let response = core
        .room_members_snapshot(synara_core::core_api::MatrixRoomMembersSnapshotRequest { room_id })
        .await
        .map_err(map_members_core_error)?;
    Ok(response)
}

pub(crate) async fn room_power_levels_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomPowerLevelsSnapshot, MatrixAuthCommandError> {
    core.room_power_levels_snapshot(synara_core::core_api::MatrixRoomMembersSnapshotRequest {
        room_id,
    })
    .await
    .map_err(map_members_core_error)
}

pub(crate) async fn room_creators_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomCreatorsSnapshot, MatrixAuthCommandError> {
    core.room_creators_snapshot(synara_core::core_api::MatrixRoomMembersSnapshotRequest { room_id })
        .await
        .map_err(map_members_core_error)
}

pub(crate) async fn room_power_level_tags_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomPowerLevelTagsSnapshot, MatrixAuthCommandError> {
    core.room_power_level_tags_snapshot(synara_core::core_api::MatrixRoomMembersSnapshotRequest {
        room_id,
    })
    .await
    .map_err(map_members_core_error)
}

fn map_members_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-rooms-members-read-members-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-rooms-members-read-room-not-found" => (
                    "NotFound",
                    "The native Matrix room members are unavailable.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix room members request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let message = match diagnostic {
                "v-rooms-members-read-power-levels-malformed"
                | "v-rooms-members-read-power-levels-too-large" => {
                    "The native Matrix room power levels are unavailable."
                }
                "v-rooms-members-read-power-level-tags-malformed"
                | "v-rooms-members-read-power-level-tags-too-large" => {
                    "The native Matrix room power-level tags are unavailable."
                }
                "v-rooms-members-read-creators-malformed" => {
                    "The native Matrix room creators are unavailable."
                }
                "v-rooms-members-read-state-failed" | "v-rooms-members-read-state-malformed" => {
                    "The native Matrix room state is unavailable."
                }
                _ => "The native Matrix room members are unavailable.",
            };
            MatrixAuthCommandError::new("Unknown", message, diagnostic)
        }
    }
}
