//! Credential-free power-level write validation.
//!
//! Live Client send/readback stays on the attached room-profile owner.

use crate::transport::{MAX_ENVELOPE_PAYLOAD_JSON_BYTES, MAX_WIRE_COUNTER};

pub const MAX_POWER_LEVEL_CONTENT_JSON_BYTES: usize = MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
pub const MAX_POWER_LEVEL_TEXT_BYTES: usize = 4 * 1024;

pub fn validate_room_power_levels_content(content: &serde_json::Value) -> Result<(), &'static str> {
    validate_power_level_payload_size(content)?;
    let Some(content) = content.as_object() else {
        return Err("v-rooms-power-levels-invalid-content");
    };

    const POWER_LEVEL_FIELDS: &[&str] = &[
        "ban",
        "events_default",
        "historical",
        "invite",
        "kick",
        "redact",
        "state_default",
        "users_default",
    ];
    for field in POWER_LEVEL_FIELDS {
        if let Some(value) = content.get(*field) {
            validate_matrix_power_level(value)?;
        }
    }
    for field in ["events", "notifications", "users"] {
        if let Some(value) = content.get(field) {
            validate_power_level_map(value)?;
        }
    }
    Ok(())
}

pub fn validate_power_level_tags_content(content: &serde_json::Value) -> Result<(), &'static str> {
    validate_power_level_payload_size(content)?;
    let Some(tags) = content.as_object() else {
        return Err("v-rooms-power-levels-invalid-content");
    };
    for (power, value) in tags {
        let parsed_power = power
            .parse::<i64>()
            .ok()
            .filter(|value| value.unsigned_abs() <= MAX_WIRE_COUNTER);
        if parsed_power.map(|value| value.to_string()).as_deref() != Some(power.as_str()) {
            return Err("v-rooms-power-levels-invalid-tag-key");
        }

        let Some(tag) = value.as_object() else {
            return Err("v-rooms-power-levels-invalid-tag");
        };
        let Some(name) = tag.get("name") else {
            return Err("v-rooms-power-levels-invalid-tag-name");
        };
        if name.as_str().is_none_or(|name| name.trim().is_empty()) {
            return Err("v-rooms-power-levels-invalid-tag-name");
        }
        validate_bounded_text(name, "v-rooms-power-levels-invalid-tag-name", true)?;
        for field in tag.keys() {
            if field != "name" && field != "color" && field != "icon" {
                return Err("v-rooms-power-levels-invalid-tag");
            }
        }
        if let Some(color) = tag.get("color") {
            validate_bounded_text(color, "v-rooms-power-levels-invalid-tag-color", false)?;
        }
        if let Some(icon) = tag.get("icon") {
            validate_power_level_tag_icon(icon)?;
        }
    }
    Ok(())
}

fn validate_power_level_payload_size(content: &serde_json::Value) -> Result<(), &'static str> {
    let byte_len = serde_json::to_vec(content)
        .map_err(|_| "v-rooms-power-levels-invalid-content")?
        .len();
    if byte_len > MAX_POWER_LEVEL_CONTENT_JSON_BYTES {
        return Err("v-rooms-power-levels-content-too-large");
    }
    Ok(())
}

fn validate_matrix_power_level(value: &serde_json::Value) -> Result<(), &'static str> {
    let valid = value
        .as_i64()
        .is_some_and(|value| value.unsigned_abs() <= MAX_WIRE_COUNTER)
        || value
            .as_u64()
            .is_some_and(|value| value <= MAX_WIRE_COUNTER);
    if valid {
        Ok(())
    } else {
        Err("v-rooms-power-levels-invalid-power")
    }
}

fn validate_power_level_map(value: &serde_json::Value) -> Result<(), &'static str> {
    let Some(map) = value.as_object() else {
        return Err("v-rooms-power-levels-invalid-power-map");
    };
    for value in map.values() {
        validate_matrix_power_level(value)?;
    }
    Ok(())
}

fn validate_bounded_text(
    value: &serde_json::Value,
    diagnostic_id: &'static str,
    required: bool,
) -> Result<(), &'static str> {
    let Some(value) = value.as_str() else {
        return Err(diagnostic_id);
    };
    if value.len() > MAX_POWER_LEVEL_TEXT_BYTES || (required && value.is_empty()) {
        return Err(diagnostic_id);
    }
    Ok(())
}

fn validate_power_level_tag_icon(value: &serde_json::Value) -> Result<(), &'static str> {
    let Some(icon) = value.as_object() else {
        return Err("v-rooms-power-levels-invalid-icon");
    };
    for field in icon.keys() {
        if field != "key" && field != "info" {
            return Err("v-rooms-power-levels-invalid-icon-field");
        }
    }
    if let Some(key) = icon.get("key") {
        validate_bounded_text(key, "v-rooms-power-levels-invalid-icon", false)?;
    }
    if let Some(info) = icon.get("info") {
        let Some(info) = info.as_object() else {
            return Err("v-rooms-power-levels-invalid-icon-info");
        };
        for (field, value) in info {
            match field.as_str() {
                "w" | "h" | "size" => {
                    let valid = value
                        .as_u64()
                        .is_some_and(|value| value <= MAX_WIRE_COUNTER);
                    if !valid {
                        return Err("v-rooms-power-levels-invalid-icon-info");
                    }
                }
                "mimetype" | "xyz.amorgan.blurhash" => {
                    validate_bounded_text(value, "v-rooms-power-levels-invalid-icon-info", false)?;
                }
                _ => return Err("v-rooms-power-levels-invalid-icon-field"),
            }
        }
    }
    Ok(())
}
