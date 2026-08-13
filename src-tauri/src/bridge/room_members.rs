//! Desktop bridges for members/power-level snapshots through `Core::command`.

use synara_core::app::members::{
    NativeRoomCreatorsSnapshot, NativeRoomMembersSnapshot, NativeRoomPowerLevelTagsSnapshot,
    NativeRoomPowerLevelsSnapshot, ROOM_CREATE_EVENT_TYPE, ROOM_POWER_LEVELS_EVENT_TYPE,
    ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_members_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomMembersSnapshot, MatrixAuthCommandError> {
    let response = dispatch(core, "matrix_room_members_snapshot", room_id).await?;
    serde_json::from_value(response).map_err(|_| members_response_error())
}

pub(crate) async fn room_power_levels_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomPowerLevelsSnapshot, MatrixAuthCommandError> {
    parse_state_snapshot(
        dispatch(core, "matrix_room_power_levels_snapshot", room_id).await?,
        ROOM_POWER_LEVELS_EVENT_TYPE,
        |room_id, session_generation, content, _| NativeRoomPowerLevelsSnapshot {
            status: "ok",
            session_generation,
            room_id,
            event_type: ROOM_POWER_LEVELS_EVENT_TYPE,
            state_key: "",
            content,
        },
    )
}

pub(crate) async fn room_creators_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomCreatorsSnapshot, MatrixAuthCommandError> {
    parse_creators_snapshot(dispatch(core, "matrix_room_creators_snapshot", room_id).await?)
}

pub(crate) async fn room_power_level_tags_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomPowerLevelTagsSnapshot, MatrixAuthCommandError> {
    parse_state_snapshot(
        dispatch(core, "matrix_room_power_level_tags_snapshot", room_id).await?,
        ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
        |room_id, session_generation, content, _| NativeRoomPowerLevelTagsSnapshot {
            status: "ok",
            session_generation,
            room_id,
            event_type: ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
            state_key: "",
            content,
        },
    )
}

async fn dispatch(
    core: &Core,
    command: &str,
    room_id: String,
) -> Result<serde_json::Value, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({ "roomId": room_id }),
    })
    .await
    .map(|response| response.payload)
    .map_err(map_members_core_error)
}

fn parse_state_snapshot<T>(
    payload: serde_json::Value,
    event_type: &'static str,
    build: fn(String, u64, serde_json::Value, &'static str) -> T,
) -> Result<T, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        session_generation: u64,
        content: serde_json::Value,
        event_type: String,
        state_key: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| members_response_error())?;
    if wire.event_type != event_type || wire.state_key != "" {
        return Err(members_response_error());
    }
    Ok(build(
        wire.room_id,
        wire.session_generation,
        wire.content,
        event_type,
    ))
}

fn parse_creators_snapshot(
    payload: serde_json::Value,
) -> Result<NativeRoomCreatorsSnapshot, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        session_generation: u64,
        event_type: String,
        state_key: String,
        creators: Vec<String>,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| members_response_error())?;
    if wire.event_type != ROOM_CREATE_EVENT_TYPE || wire.state_key != "" {
        return Err(members_response_error());
    }
    Ok(NativeRoomCreatorsSnapshot {
        status: "ok",
        session_generation: wire.session_generation,
        room_id: wire.room_id,
        event_type: ROOM_CREATE_EVENT_TYPE,
        state_key: "",
        creators: wire.creators,
    })
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

fn members_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room members are unavailable.",
        "v-rooms-members-read-members-failed",
    )
}
