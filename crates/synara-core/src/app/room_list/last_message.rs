//! Privacy-safe last-message preview for room-list rows.
//!
//! Extracts a short body from the SDK latest-event JSON. No tokens, no
//! `mxc://`, and no invention when the event has no displayable text.

use serde_json::Value as JsonValue;

pub const LAST_MESSAGE_PREVIEW_MAX_CHARS: usize = 160;

/// Collapse whitespace, strip media URIs, and bound the preview.
pub fn sanitize_last_message_preview(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len().min(LAST_MESSAGE_PREVIEW_MAX_CHARS));
    let mut previous_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !previous_space && !out.is_empty() {
                out.push(' ');
                previous_space = true;
            }
            continue;
        }
        previous_space = false;
        out.push(ch);
        if out.chars().count() >= LAST_MESSAGE_PREVIEW_MAX_CHARS {
            break;
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("mxc://") || trimmed.contains("syt_") || trimmed.contains("access_token") {
        return media_label_for_body(trimmed);
    }
    Some(trimmed.to_owned())
}

pub fn last_message_preview_from_invite(inviter: Option<&str>) -> Option<String> {
    match inviter.map(str::trim).filter(|value| !value.is_empty()) {
        Some(user) if !user.contains("mxc://") && !user.contains("syt_") => {
            sanitize_last_message_preview(&format!("Invited by {user}"))
        }
        _ => Some("Invited".to_owned()),
    }
}

pub fn last_message_preview_from_event_json_str(raw: &str) -> Option<String> {
    serde_json::from_str(raw)
        .ok()
        .and_then(|value| last_message_preview_from_event_json(&value))
}

pub fn last_message_preview_from_event_json(value: &JsonValue) -> Option<String> {
    let event_type = value.get("type").and_then(JsonValue::as_str)?;
    let content = value.get("content").unwrap_or(&JsonValue::Null);
    match event_type {
        "m.room.message" => preview_from_room_message(content),
        "m.sticker" => Some("Sticker".to_owned()),
        "m.room.encrypted" => Some("Encrypted message".to_owned()),
        "m.poll.start" | "org.matrix.msc3381.poll.start" => preview_from_poll(content)
            .or_else(|| Some("Poll".to_owned())),
        "m.room.member" => preview_from_membership(value, content),
        "m.call.invite" | "m.call.notify" => Some("Call".to_owned()),
        "m.room.topic" => content
            .get("topic")
            .and_then(JsonValue::as_str)
            .and_then(sanitize_last_message_preview)
            .or_else(|| Some("Topic updated".to_owned())),
        "m.room.name" => content
            .get("name")
            .and_then(JsonValue::as_str)
            .and_then(sanitize_last_message_preview)
            .or_else(|| Some("Name updated".to_owned())),
        _ => None,
    }
}

fn preview_from_room_message(content: &JsonValue) -> Option<String> {
    match content.get("msgtype").and_then(JsonValue::as_str) {
        Some("m.image") => Some("Image".to_owned()),
        Some("m.video") => Some("Video".to_owned()),
        Some("m.audio") => Some("Audio".to_owned()),
        Some("m.file") => Some("File".to_owned()),
        Some("m.location") => Some("Location".to_owned()),
        Some("m.text") | Some("m.notice") | Some("m.emote") | Some(_) | None => content
            .get("body")
            .and_then(JsonValue::as_str)
            .and_then(sanitize_last_message_preview)
            .or_else(|| Some("Message".to_owned())),
    }
}

fn preview_from_poll(content: &JsonValue) -> Option<String> {
    content
        .pointer("/org.matrix.msc3381.poll.start/question/org.matrix.msc1767.text")
        .or_else(|| content.pointer("/m.poll.start/question/m.text"))
        .or_else(|| content.pointer("/org.matrix.msc3381.poll.start/question/body"))
        .and_then(JsonValue::as_str)
        .and_then(sanitize_last_message_preview)
}

fn preview_from_membership(event: &JsonValue, content: &JsonValue) -> Option<String> {
    let membership = content.get("membership").and_then(JsonValue::as_str)?;
    let name = content
        .get("displayname")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains("mxc://"))
        .or_else(|| {
            event
                .get("state_key")
                .and_then(JsonValue::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty() && !value.contains("mxc://"))
        })
        .unwrap_or("Someone");
    let verb = match membership {
        "join" => "joined",
        "leave" => "left",
        "invite" => "was invited",
        "ban" => "was banned",
        "knock" => "requested to join",
        _ => return None,
    };
    sanitize_last_message_preview(&format!("{name} {verb}"))
}

fn media_label_for_body(text: &str) -> Option<String> {
    if text.contains("mxc://") {
        Some("Media".to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_trims_and_bounds_without_mxc_or_token() {
        assert_eq!(
            sanitize_last_message_preview("  hello   world  "),
            Some("hello world".into())
        );
        assert_eq!(sanitize_last_message_preview("   "), None);
        let long = "a".repeat(200);
        let sanitized = sanitize_last_message_preview(&long).expect("bounded");
        assert_eq!(sanitized.chars().count(), LAST_MESSAGE_PREVIEW_MAX_CHARS);
        assert_eq!(
            sanitize_last_message_preview("see mxc://example.org/abc"),
            Some("Media".into())
        );
        assert_eq!(sanitize_last_message_preview("syt_secret"), None);
        assert_eq!(sanitize_last_message_preview("access_token=abc"), None);
    }

    #[test]
    fn text_message_uses_body() {
        let event = json!({
            "type": "m.room.message",
            "content": { "msgtype": "m.text", "body": "Hello from Alice" }
        });
        assert_eq!(
            last_message_preview_from_event_json(&event).as_deref(),
            Some("Hello from Alice")
        );
    }

    #[test]
    fn media_and_sticker_use_labels() {
        assert_eq!(
            last_message_preview_from_event_json(&json!({
                "type": "m.room.message",
                "content": { "msgtype": "m.image", "body": "mxc://example.org/img", "url": "mxc://example.org/img" }
            }))
            .as_deref(),
            Some("Image")
        );
        assert_eq!(
            last_message_preview_from_event_json(&json!({
                "type": "m.sticker",
                "content": { "body": "sticker", "url": "mxc://example.org/sticker" }
            }))
            .as_deref(),
            Some("Sticker")
        );
        assert_eq!(
            last_message_preview_from_event_json(&json!({
                "type": "m.room.encrypted",
                "content": { "algorithm": "m.megolm.v1.aes-sha2" }
            }))
            .as_deref(),
            Some("Encrypted message")
        );
    }

    #[test]
    fn membership_and_invite_are_privacy_safe() {
        assert_eq!(
            last_message_preview_from_event_json(&json!({
                "type": "m.room.member",
                "state_key": "@alice:example.org",
                "content": { "membership": "join", "displayname": "Alice" }
            }))
            .as_deref(),
            Some("Alice joined")
        );
        assert_eq!(
            last_message_preview_from_invite(Some("@alex:example.org")).as_deref(),
            Some("Invited by @alex:example.org")
        );
        assert_eq!(
            last_message_preview_from_invite(Some("syt_secret")).as_deref(),
            Some("Invited")
        );
        let raw = last_message_preview_from_event_json(&json!({
            "type": "m.room.message",
            "content": { "msgtype": "m.text", "body": "ok" }
        }))
        .expect("body");
        assert!(!raw.contains("mxc://"));
        assert!(!raw.contains("syt_"));
        assert!(!raw.contains("password"));
    }

    #[test]
    fn unknown_types_are_not_invented() {
        assert_eq!(
            last_message_preview_from_event_json(&json!({
                "type": "m.reaction",
                "content": { "m.relates_to": { "key": "👍" } }
            })),
            None
        );
    }
}
