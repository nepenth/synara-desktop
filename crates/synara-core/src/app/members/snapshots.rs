//! Credential-free members / power-level snapshot helpers.
//!
//! Live Client member/state I/O stays on the attached room-profile owner.

use std::collections::BTreeSet;

use matrix_sdk::ruma::{
    events::room::member::MembershipState, events::room::power_levels::UserPowerLevel, OwnedRoomId,
    OwnedUserId, UserId,
};

use crate::dto::{Membership, RoomMember};
use crate::transport::MAX_WIRE_COUNTER;

use super::{validate_power_level_tags_content, MAX_POWER_LEVEL_CONTENT_JSON_BYTES};

pub fn parse_room_members_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    room_id
        .trim()
        .parse()
        .map_err(|_| "v-rooms-members-read-invalid-room")
}

pub fn project_room_member(
    room_id: &OwnedRoomId,
    member: &matrix_sdk::room::RoomMember,
    is_two_party_direct: bool,
    current_user: Option<&UserId>,
) -> Result<RoomMember, &'static str> {
    let membership = match member.membership() {
        MembershipState::Ban => Membership::Ban,
        MembershipState::Invite => Membership::Invite,
        MembershipState::Join => Membership::Join,
        MembershipState::Knock => Membership::Knock,
        MembershipState::Leave => Membership::Leave,
        _ => return Err("v-rooms-members-read-unsupported-membership"),
    };
    let power_level = match member.power_level() {
        UserPowerLevel::Infinite => i32::MAX,
        UserPowerLevel::Int(value) => {
            i32::try_from(value).map_err(|_| "v-rooms-members-read-power-level-invalid")?
        }
        _ => return Err("v-rooms-members-read-power-level-invalid"),
    };

    Ok(RoomMember {
        room_id: room_id.to_string(),
        user_id: member.user_id().to_string(),
        display_name: member.display_name().map(ToOwned::to_owned),
        avatar_url: member.avatar_url().map(ToString::to_string),
        membership,
        power_level,
        is_direct_target: is_two_party_direct
            .then(|| current_user.is_some_and(|current_user| current_user != member.user_id())),
    })
}

pub fn project_room_creators(event: &serde_json::Value) -> Result<Vec<String>, &'static str> {
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

pub fn validate_power_levels_snapshot_content(
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

pub fn validate_power_level_tags_snapshot_content(
    content: &serde_json::Value,
) -> Result<(), &'static str> {
    validate_power_level_tags_content(content).map_err(|diagnostic| {
        if diagnostic == "v-rooms-power-levels-content-too-large" {
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
