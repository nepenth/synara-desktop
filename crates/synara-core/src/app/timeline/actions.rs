//! Typed native timeline action commands (edit / redact / forward).
//!
//! Reply transport remains `matrix_send_text` with `reply_to`. Reactions stay
//! on the V-SEND.2 owner. These DTOs are room-addressed so the still-active
//! legacy presenter can re-home affordances without selecting the native
//! timeline presenter.

use serde::{Deserialize, Serialize};

/// Version of the bounded timeline-action readback contract.
pub const NATIVE_TIMELINE_ACTION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTimelineActionKind {
    EditText,
    Redact,
    ForwardText,
    ForwardMedia,
    Report,
    Pin,
    Unpin,
    PollVote,
    CallDecline,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineEditTextRequest {
    pub room_id: String,
    pub event_id: String,
    pub body: String,
    /// Optional Matrix HTML body (`org.matrix.custom.html`).
    #[serde(default)]
    pub formatted_body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReportRequest {
    pub room_id: String,
    pub event_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelinePinRequest {
    pub room_id: String,
    pub event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineRedactRequest {
    pub room_id: String,
    pub event_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineForwardTextRequest {
    pub source_room_id: String,
    pub event_id: String,
    pub target_room_id: String,
    #[serde(default)]
    pub as_quote: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineForwardMediaRequest {
    pub source_room_id: String,
    pub event_id: String,
    pub target_room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelinePollVoteRequest {
    pub room_id: String,
    pub event_id: String,
    /// Selected answer ids for the poll start event. Empty clears the vote.
    #[serde(default)]
    pub answer_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineCallDeclineRequest {
    pub room_id: String,
    /// `m.rtc.notification` event id to decline.
    pub event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineActionReadback {
    pub schema_version: u32,
    pub action: NativeTimelineActionKind,
    pub room_id: String,
    /// For edit/forward: the newly sent event id. For redact: the redacted event id.
    pub event_id: String,
    pub status: &'static str,
}

impl<'de> Deserialize<'de> for NativeTimelineActionReadback {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct NativeTimelineActionReadbackHelper {
            schema_version: u32,
            action: NativeTimelineActionKind,
            room_id: String,
            event_id: String,
            status: String,
        }

        let helper = NativeTimelineActionReadbackHelper::deserialize(deserializer)?;
        Ok(Self {
            schema_version: helper.schema_version,
            action: helper.action,
            room_id: helper.room_id,
            event_id: helper.event_id,
            status: intern_action_status(&helper.status)?,
        })
    }
}

fn intern_action_status<E: serde::de::Error>(value: &str) -> Result<&'static str, E> {
    match value {
        "sent" => Ok("sent"),
        "redacted" => Ok("redacted"),
        "reported" => Ok("reported"),
        "pinned" => Ok("pinned"),
        "unpinned" => Ok("unpinned"),
        "already_pinned" => Ok("already_pinned"),
        "already_unpinned" => Ok("already_unpinned"),
        "voted" => Ok("voted"),
        "declined" => Ok("declined"),
        other => Err(E::unknown_variant(
            other,
            &[
                "sent",
                "redacted",
                "reported",
                "pinned",
                "unpinned",
                "already_pinned",
                "already_unpinned",
                "voted",
                "declined",
            ],
        )),
    }
}

/// Choose whether outbound Matrix HTML should be attached beside plain text.
pub fn should_attach_formatted_body(body: &str, formatted_body: Option<&str>) -> bool {
    match formatted_body
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(html) => html != body.trim(),
        None => false,
    }
}

/// Build the plain-text body used when forwarding a text-like message.
pub fn format_forwarded_plain_body(sender_label: &str, body: &str, as_quote: bool) -> String {
    let trimmed = trim_mx_reply_prefix(body.trim());
    if as_quote {
        if trimmed.is_empty() {
            format!("> <{sender_label}>")
        } else {
            let quoted = trimmed
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("> <{sender_label}>\n{quoted}")
        }
    } else if trimmed.is_empty() {
        format!("Forwarded from {sender_label}")
    } else {
        format!("Forwarded from {sender_label}\n\n{trimmed}")
    }
}

/// Attribute a media/sticker body when forwarding without rewriting media sources.
pub fn format_forwarded_media_body(sender_label: &str, body: &str) -> String {
    format_forwarded_plain_body(sender_label, body, false)
}

fn trim_mx_reply_prefix(body: &str) -> String {
    const MARKER: &str = "\n\n";
    if body.starts_with("> <") {
        if let Some(index) = body.find(MARKER) {
            return body[index + MARKER.len()..].to_owned();
        }
    }
    body.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatted_body_attaches_only_when_it_differs_from_plain_text() {
        assert!(!should_attach_formatted_body("hello", Some("hello")));
        assert!(!should_attach_formatted_body("hello", Some("  ")));
        assert!(should_attach_formatted_body("hello", Some("<p>hello</p>")));
    }

    #[test]
    fn forward_plain_and_quote_bodies_attribute_the_source_sender() {
        assert_eq!(
            format_forwarded_plain_body("@alice:example.org", "hello", false),
            "Forwarded from @alice:example.org\n\nhello"
        );
        assert_eq!(
            format_forwarded_plain_body("@alice:example.org", "hello\nthere", true),
            "> <@alice:example.org>\n> hello\n> there"
        );
    }

    #[test]
    fn action_request_schemas_stay_room_addressed() {
        let edit: NativeTimelineEditTextRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$edit:example.org",
            "body": "updated"
        }))
        .unwrap();
        assert_eq!(edit.event_id, "$edit:example.org");

        let media: NativeTimelineForwardMediaRequest = serde_json::from_value(serde_json::json!({
            "sourceRoomId": "!source:example.org",
            "eventId": "$media:example.org",
            "targetRoomId": "!target:example.org"
        }))
        .unwrap();
        assert_eq!(media.event_id, "$media:example.org");
        assert_eq!(
            format_forwarded_media_body("@alice:example.org", "photo.jpg"),
            "Forwarded from @alice:example.org\n\nphoto.jpg"
        );

        let vote: NativeTimelinePollVoteRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$poll:example.org",
            "answerIds": ["a1", "a2"]
        }))
        .unwrap();
        assert_eq!(vote.answer_ids, vec!["a1", "a2"]);

        let decline: NativeTimelineCallDeclineRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$rtc:example.org"
        }))
        .unwrap();
        assert_eq!(decline.event_id, "$rtc:example.org");

        let redact: NativeTimelineRedactRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$redact:example.org",
            "reason": "spam"
        }))
        .unwrap();
        assert_eq!(redact.reason.as_deref(), Some("spam"));

        let forward: NativeTimelineForwardTextRequest = serde_json::from_value(serde_json::json!({
            "sourceRoomId": "!source:example.org",
            "eventId": "$fwd:example.org",
            "targetRoomId": "!target:example.org",
            "asQuote": true
        }))
        .unwrap();
        assert!(forward.as_quote);

        let readback = NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::EditText,
            room_id: "!room:example.org".into(),
            event_id: "$new:example.org".into(),
            status: "sent",
        };
        let json = serde_json::to_value(readback).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["action"], "edit_text");
        assert_eq!(json["status"], "sent");
        let decoded: NativeTimelineActionReadback = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.status, "sent");
        assert_eq!(decoded.action, NativeTimelineActionKind::EditText);
    }
}
