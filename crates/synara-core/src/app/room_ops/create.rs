//! Native room-create request builder.
//!
//! Live `Client::create_room` stays on the attached room-profile owner.

use matrix_sdk::ruma::{
    api::client::room::{
        create_room::{self, v3::RoomPreset},
        Visibility,
    },
    events::AnyInitialStateEvent,
    serde::Raw,
    OwnedRoomId, OwnedUserId, RoomVersionId,
};

use super::{
    MatrixRoomCreateContent, MatrixRoomCreatePowerLevels, MatrixRoomCreatePreset,
    MatrixRoomCreateRequest, MatrixRoomCreateVisibility,
    encrypted_state::{
        encrypted_state_events_setting_enabled, is_call_room_type, room_encryption_content,
        should_create_with_encrypted_state,
    },
};

pub fn build_room_create_request(
    input: MatrixRoomCreateRequest,
) -> Result<create_room::v3::Request, &'static str> {
    build_room_create_request_with_setting(input, encrypted_state_events_setting_enabled())
}

pub(crate) fn build_room_create_request_with_setting(
    input: MatrixRoomCreateRequest,
    setting_enabled: bool,
) -> Result<create_room::v3::Request, &'static str> {
    let MatrixRoomCreateRequest {
        name,
        topic,
        room_version,
        room_alias_name,
        is_direct,
        invite,
        visibility,
        preset,
        creation_content,
        encryption,
        encrypt_state_events,
        join_rule,
        knock,
        parent_room_id,
        power_level_content_override,
    } = input;

    let name = name
        .map(|value| {
            let value = value.trim();
            if value.is_empty() || value.chars().count() > 255 {
                return Err("v-rooms-room-create-invalid-name");
            }
            Ok(value.to_owned())
        })
        .transpose()?;
    let topic = topic
        .map(|value| {
            let value = value.trim();
            if value.chars().count() > 2_048 {
                return Err("v-rooms-room-create-invalid-topic");
            }
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_owned()))
            }
        })
        .transpose()?
        .flatten();
    let room_version = room_version
        .map(|value| {
            value
                .trim()
                .parse::<RoomVersionId>()
                .map_err(|_| "v-rooms-room-create-invalid-room-version")
        })
        .transpose()?;
    let room_alias_name = room_alias_name
        .map(|value| {
            let value = value.trim();
            if value.is_empty() || value.chars().count() > 255 {
                return Err("v-rooms-room-create-invalid-alias");
            }
            Ok(value.to_owned())
        })
        .transpose()?;
    let invite = invite
        .into_iter()
        .map(|value| {
            value
                .trim()
                .parse::<OwnedUserId>()
                .map_err(|_| "v-rooms-room-create-invalid-invite")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let parent_room_id = parent_room_id
        .map(|value| {
            value
                .trim()
                .parse::<OwnedRoomId>()
                .map_err(|_| "v-rooms-room-create-invalid-parent")
        })
        .transpose()?;

    let room_type = creation_content
        .as_ref()
        .and_then(|content| content.room_type.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if creation_content
        .as_ref()
        .and_then(|content| content.room_type.as_deref())
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err("v-rooms-room-create-invalid-creation-content");
    }

    let creation_content = creation_content
        .map(build_room_create_creation_content)
        .transpose()?;
    let power_level_content_override = power_level_content_override
        .map(build_room_create_power_levels)
        .transpose()?;

    let mut initial_state = Vec::new();
    if encryption {
        let encrypt_state = should_create_with_encrypted_state(
            encryption,
            encrypt_state_events,
            setting_enabled,
            is_call_room_type(room_type.as_deref()),
        );
        initial_state.push(raw_room_create_state(
            "m.room.encryption",
            "",
            room_encryption_content(encrypt_state),
        )?);
    }
    if room_type.as_deref() == Some("org.matrix.msc3417.call") {
        initial_state.push(raw_room_create_state(
            "org.matrix.msc3401.call",
            "",
            serde_json::json!({}),
        )?);
    }
    if let Some(join_rules) =
        build_room_create_join_rules(join_rule.as_deref(), knock, parent_room_id.as_ref())?
    {
        initial_state.push(join_rules);
    }

    let mut request = create_room::v3::Request::new();
    request.name = name;
    request.topic = topic;
    request.room_version = room_version;
    request.room_alias_name = room_alias_name;
    request.is_direct = is_direct;
    request.invite = invite;
    request.visibility = match visibility {
        Some(MatrixRoomCreateVisibility::Public) => Visibility::Public,
        Some(MatrixRoomCreateVisibility::Private) | None => Visibility::Private,
    };
    request.preset = preset.map(|preset| match preset {
        MatrixRoomCreatePreset::Private => RoomPreset::PrivateChat,
        MatrixRoomCreatePreset::Public => RoomPreset::PublicChat,
        MatrixRoomCreatePreset::TrustedPrivate => RoomPreset::TrustedPrivateChat,
    });
    request.creation_content = creation_content;
    request.initial_state = initial_state;
    request.power_level_content_override = power_level_content_override;
    Ok(request)
}

fn build_room_create_creation_content(
    content: MatrixRoomCreateContent,
) -> Result<Raw<create_room::v3::CreationContent>, &'static str> {
    let mut value = serde_json::Map::new();
    if let Some(room_type) = content.room_type {
        value.insert("type".to_owned(), serde_json::Value::String(room_type));
    }
    if let Some(federate) = content.federate {
        value.insert("m.federate".to_owned(), serde_json::json!(federate));
    }
    if let Some(additional_creators) = content.additional_creators {
        let additional_creators = additional_creators
            .into_iter()
            .map(|value| {
                value
                    .trim()
                    .parse::<OwnedUserId>()
                    .map_err(|_| "v-rooms-room-create-invalid-additional-creator")
            })
            .collect::<Result<Vec<_>, _>>()?;
        value.insert(
            "additional_creators".to_owned(),
            serde_json::to_value(additional_creators).expect("user IDs serialize"),
        );
    }
    raw_room_create(
        serde_json::Value::Object(value),
        "v-rooms-room-create-invalid-creation-content",
    )
}

fn build_room_create_power_levels(
    power_levels: MatrixRoomCreatePowerLevels,
) -> Result<Raw<create_room::RoomPowerLevelsContentOverride>, &'static str> {
    if power_levels.events_default.is_none() && power_levels.events.is_empty() {
        return Err("v-rooms-room-create-invalid-power-level");
    }
    let mut value = serde_json::Map::new();
    if let Some(events_default) = power_levels.events_default {
        value.insert(
            "events_default".to_owned(),
            serde_json::json!(events_default),
        );
    }
    if !power_levels.events.is_empty() {
        value.insert(
            "events".to_owned(),
            serde_json::to_value(power_levels.events).expect("power level map serializes"),
        );
    }
    raw_room_create(
        serde_json::Value::Object(value),
        "v-rooms-room-create-invalid-power-level",
    )
}

fn build_room_create_join_rules(
    join_rule: Option<&str>,
    knock: bool,
    parent_room_id: Option<&OwnedRoomId>,
) -> Result<Option<Raw<AnyInitialStateEvent>>, &'static str> {
    let Some(join_rule) = join_rule else {
        if knock {
            return Err("v-rooms-room-create-invalid-join-rule");
        }
        return Ok(None);
    };

    let join_rule = join_rule.trim();
    let join_rule = match join_rule {
        "invite" | "knock" => {
            if join_rule == "knock" || knock {
                "knock"
            } else {
                "invite"
            }
        }
        "restricted" | "knock_restricted" => {
            if join_rule == "knock_restricted" || knock {
                "knock_restricted"
            } else {
                "restricted"
            }
        }
        "public" if !knock => "public",
        _ => return Err("v-rooms-room-create-invalid-join-rule"),
    };

    let restricted = matches!(join_rule, "restricted" | "knock_restricted");
    if restricted && parent_room_id.is_none() {
        return Err("v-rooms-room-create-missing-restricted-parent");
    }

    let mut content = serde_json::json!({ "join_rule": join_rule });
    if restricted {
        content["allow"] = serde_json::json!([{
            "type": "m.room_membership",
            "room_id": parent_room_id.expect("restricted parent checked").to_string(),
        }]);
    }
    Ok(Some(raw_room_create_state(
        "m.room.join_rules",
        "",
        content,
    )?))
}

fn raw_room_create_state(
    event_type: &str,
    state_key: &str,
    content: serde_json::Value,
) -> Result<Raw<AnyInitialStateEvent>, &'static str> {
    raw_room_create(
        serde_json::json!({
            "type": event_type,
            "state_key": state_key,
            "content": content,
        }),
        "v-rooms-room-create-invalid-creation-content",
    )
}

fn raw_room_create<T>(
    value: serde_json::Value,
    diagnostic_id: &'static str,
) -> Result<Raw<T>, &'static str> {
    serde_json::value::to_raw_value(&value)
        .map(Raw::<T>::from_json)
        .map_err(|_| diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::room_ops::ipc::MatrixRoomCreateContent;

    fn sample(
        encryption: bool,
        encrypt_state_events: bool,
        room_type: Option<&str>,
    ) -> MatrixRoomCreateRequest {
        MatrixRoomCreateRequest {
            name: Some("Room".into()),
            topic: None,
            room_version: None,
            room_alias_name: None,
            is_direct: false,
            invite: vec![],
            visibility: None,
            preset: None,
            creation_content: room_type.map(|room_type| MatrixRoomCreateContent {
                room_type: Some(room_type.to_owned()),
                federate: None,
                additional_creators: None,
            }),
            encryption,
            encrypt_state_events,
            join_rule: None,
            knock: false,
            parent_room_id: None,
            power_level_content_override: None,
        }
    }

    fn encryption_event(request: &create_room::v3::Request) -> Option<serde_json::Value> {
        request.initial_state.iter().find_map(|event| {
            let value: serde_json::Value = serde_json::from_str(event.json().get()).ok()?;
            (value.get("type")?.as_str()? == "m.room.encryption")
                .then_some(value["content"].clone())
        })
    }

    #[test]
    fn encryption_and_flag_emit_stable_and_unstable_keys() {
        let request =
            build_room_create_request_with_setting(sample(true, true, None), true).expect("create");
        let content = encryption_event(&request).expect("encryption event");
        assert_eq!(content["algorithm"], "m.megolm.v1.aes-sha2");
        assert_eq!(content["encrypt_state_events"], true);
        assert_eq!(content["io.element.msc4362.encrypt_state_events"], true);
    }

    #[test]
    fn encryption_false_never_adds_the_event() {
        let request = build_room_create_request_with_setting(sample(false, true, None), true)
            .expect("create");
        assert!(encryption_event(&request).is_none());
    }

    #[test]
    fn setting_off_backstop_rejects_a_true_flag() {
        let request = build_room_create_request_with_setting(sample(true, true, None), false)
            .expect("create");
        let content = encryption_event(&request).expect("encryption event");
        assert!(content.get("encrypt_state_events").is_none());
        assert!(
            content
                .get("io.element.msc4362.encrypt_state_events")
                .is_none()
        );
    }

    #[test]
    fn call_rooms_never_get_the_flag() {
        for room_type in ["org.matrix.msc3417.call", "m.call"] {
            let request =
                build_room_create_request_with_setting(sample(true, true, Some(room_type)), true)
                    .expect("create");
            let content = encryption_event(&request).expect("encryption event");
            assert!(content.get("encrypt_state_events").is_none(), "{room_type}");
        }
    }
}
