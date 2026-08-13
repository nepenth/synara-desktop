use super::*;

pub use synara_core::app::members::{
    NativeRoomCreatorsSnapshot, NativeRoomPowerLevelTagsSnapshot, NativeRoomPowerLevelsSnapshot,
    ROOM_CREATE_EVENT_TYPE, ROOM_POWER_LEVELS_EVENT_TYPE, ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};

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

#[tauri::command]
pub async fn matrix_room_power_levels_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeRoomPowerLevelsSnapshot, MatrixAuthCommandError> {
    let room_id = parse_room_members_room_id(&room_id).map_err(map_room_members_error)?;
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let room = active
        .client
        .get_room(&room_id)
        .ok_or_else(|| map_room_members_error("v-rooms-members-read-room-not-found"))?;
    let content = read_room_state_content(&room, ROOM_POWER_LEVELS_EVENT_TYPE)
        .await
        .map_err(map_room_members_error)?
        .unwrap_or_else(|| serde_json::json!({}));
    validate_power_levels_snapshot_content(&content).map_err(map_room_members_error)?;

    Ok(NativeRoomPowerLevelsSnapshot {
        status: "ok",
        session_generation: active.sync.session_generation(),
        room_id: room_id.to_string(),
        event_type: ROOM_POWER_LEVELS_EVENT_TYPE,
        state_key: "",
        content,
    })
}

#[tauri::command]
pub async fn matrix_room_creators_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeRoomCreatorsSnapshot, MatrixAuthCommandError> {
    let room_id = parse_room_members_room_id(&room_id).map_err(map_room_members_error)?;
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let room = active
        .client
        .get_room(&room_id)
        .ok_or_else(|| map_room_members_error("v-rooms-members-read-room-not-found"))?;
    let Some(event) = read_room_state_event(&room, ROOM_CREATE_EVENT_TYPE)
        .await
        .map_err(map_room_members_error)?
    else {
        return Ok(NativeRoomCreatorsSnapshot {
            status: "ok",
            session_generation: active.sync.session_generation(),
            room_id: room_id.to_string(),
            event_type: ROOM_CREATE_EVENT_TYPE,
            state_key: "",
            creators: Vec::new(),
        });
    };

    let creators = project_room_creators(&event).map_err(map_room_members_error)?;
    Ok(NativeRoomCreatorsSnapshot {
        status: "ok",
        session_generation: active.sync.session_generation(),
        room_id: room_id.to_string(),
        event_type: ROOM_CREATE_EVENT_TYPE,
        state_key: "",
        creators,
    })
}

#[tauri::command]
pub async fn matrix_room_power_level_tags_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeRoomPowerLevelTagsSnapshot, MatrixAuthCommandError> {
    let room_id = parse_room_members_room_id(&room_id).map_err(map_room_members_error)?;
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let room = active
        .client
        .get_room(&room_id)
        .ok_or_else(|| map_room_members_error("v-rooms-members-read-room-not-found"))?;
    let content = read_room_state_content(&room, ROOM_POWER_LEVEL_TAGS_EVENT_TYPE)
        .await
        .map_err(map_room_members_error)?
        .unwrap_or_else(|| serde_json::json!({}));
    validate_power_level_tags_snapshot_content(&content).map_err(map_room_members_error)?;

    Ok(NativeRoomPowerLevelTagsSnapshot {
        status: "ok",
        session_generation: active.sync.session_generation(),
        room_id: room_id.to_string(),
        event_type: ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
        state_key: "",
        content,
    })
}

async fn read_room_state_content(
    room: &Room,
    event_type: &str,
) -> Result<Option<serde_json::Value>, &'static str> {
    read_room_state_event(room, event_type)
        .await
        .map(|event| event.map(|event| event["content"].clone()))
}

async fn read_room_state_event(
    room: &Room,
    event_type: &str,
) -> Result<Option<serde_json::Value>, &'static str> {
    let event = room
        .get_state_event(StateEventType::from(event_type), "")
        .await
        .map_err(|_| "v-rooms-members-read-state-failed")?;
    event
        .map(|event| {
            serde_json::to_value(event).map_err(|_| "v-rooms-members-read-state-malformed")
        })
        .transpose()
}

pub(super) fn project_room_creators(
    event: &serde_json::Value,
) -> Result<Vec<String>, &'static str> {
    let content = event
        .get("content")
        .and_then(serde_json::Value::as_object)
        .ok_or("v-rooms-members-read-creators-malformed")?;
    let room_version = content
        .get("room_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("1");

    if !creators_supported(room_version) {
        return Ok(Vec::new());
    }

    let sender = event
        .get("sender")
        .and_then(serde_json::Value::as_str)
        .ok_or("v-rooms-members-read-creators-malformed")?;
    let sender = sender
        .parse::<OwnedUserId>()
        .map_err(|_| "v-rooms-members-read-creators-malformed")?;
    let mut creators = BTreeSet::from([sender.to_string()]);
    if let Some(additional_creators) = content.get("additional_creators") {
        let additional_creators = additional_creators
            .as_array()
            .ok_or("v-rooms-members-read-creators-malformed")?;
        for creator in additional_creators {
            let creator = creator
                .as_str()
                .ok_or("v-rooms-members-read-creators-malformed")?
                .parse::<OwnedUserId>()
                .map_err(|_| "v-rooms-members-read-creators-malformed")?;
            creators.insert(creator.to_string());
        }
    }
    Ok(creators.into_iter().collect())
}

fn creators_supported(room_version: &str) -> bool {
    !matches!(
        room_version,
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" | "11"
    )
}

pub(super) fn validate_power_levels_snapshot_content(
    content: &serde_json::Value,
) -> Result<(), &'static str> {
    if serde_json::to_vec(content)
        .map_err(|_| "v-rooms-members-read-power-levels-malformed")?
        .len()
        > MAX_POWER_LEVEL_CONTENT_JSON_BYTES
    {
        return Err("v-rooms-members-read-power-levels-too-large");
    }
    let Some(content) = content.as_object() else {
        return Err("v-rooms-members-read-power-levels-malformed");
    };

    for field in [
        "ban",
        "events_default",
        "historical",
        "invite",
        "kick",
        "redact",
        "state_default",
        "users_default",
    ] {
        if let Some(value) = content.get(field) {
            validate_snapshot_power(value)?;
        }
    }
    for field in ["events", "notifications", "users"] {
        if let Some(value) = content.get(field) {
            let Some(values) = value.as_object() else {
                return Err("v-rooms-members-read-power-levels-malformed");
            };
            for value in values.values() {
                validate_snapshot_power(value)?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_power_level_tags_snapshot_content(
    content: &serde_json::Value,
) -> Result<(), &'static str> {
    super::room_ops::validate_power_level_tags_content(content).map_err(|error| {
        if error.diagnostic_id == "v-rooms-power-levels-content-too-large" {
            "v-rooms-members-read-power-level-tags-too-large"
        } else {
            "v-rooms-members-read-power-level-tags-malformed"
        }
    })
}

fn validate_snapshot_power(value: &serde_json::Value) -> Result<(), &'static str> {
    let valid = value
        .as_i64()
        .is_some_and(|value| value.unsigned_abs() <= MAX_WIRE_COUNTER)
        || value
            .as_u64()
            .is_some_and(|value| value <= MAX_WIRE_COUNTER);
    valid
        .then_some(())
        .ok_or("v-rooms-members-read-power-levels-malformed")
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
