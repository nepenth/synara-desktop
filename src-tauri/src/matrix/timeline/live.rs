//! D0.3 live Matrix SDK timeline ownership and privacy-safe projection.
//!
//! SDK timeline objects stay inside the Rust session. The webview receives a
//! product snapshot containing only stable identifiers, sender IDs,
//! event types, timestamps, and safe display text.

use std::collections::HashMap;
use std::sync::Arc;

use matrix_sdk::{ruma::OwnedRoomId, Client};
use matrix_sdk_ui::timeline::{
    Timeline, TimelineBuilder, TimelineItem as SdkTimelineItem,
    TimelineItemContent as SdkTimelineItemContent,
};
use serde::{Deserialize, Serialize};

const PAGINATION_BATCH_SIZE: u16 = 30;
const REDACTED_PLACEHOLDER: &str = "Message removed";
const UTD_PLACEHOLDER: &str = "Unable to decrypt this message";
const UNSUPPORTED_PLACEHOLDER: &str = "Unsupported event";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTimelineDirection {
    Backwards,
    Forwards,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineItem {
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub body: String,
    pub origin_server_ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub items: Vec<NativeTimelineItem>,
    pub hit_start: bool,
}

struct LiveTimelineEntry {
    timeline: Arc<Timeline>,
    hit_start: bool,
}

pub struct NativeTimelineRegistry {
    session_generation: u64,
    entries: HashMap<String, LiveTimelineEntry>,
}

impl NativeTimelineRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            entries: HashMap::new(),
        }
    }

    pub async fn open(
        &mut self,
        client: &Client,
        room_id: &str,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?;
        let room_id_string = room_id.to_string();
        if !self.entries.contains_key(&room_id_string) {
            let room = client
                .get_room(&room_id)
                .ok_or("d0.3-timeline-room-not-found")?;
            let timeline = TimelineBuilder::new(&room)
                .build()
                .await
                .map_err(|_| "d0.3-timeline-open-failed")?;
            self.entries.insert(
                room_id_string.clone(),
                LiveTimelineEntry {
                    timeline: Arc::new(timeline),
                    hit_start: false,
                },
            );
        }
        self.snapshot(&room_id_string).await
    }

    pub async fn snapshot(&self, room_id: &str) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let entry = self.entries.get(&room_id).ok_or("d0.3-timeline-not-open")?;
        snapshot_from_timeline(
            self.session_generation,
            room_id,
            &entry.timeline,
            entry.hit_start,
        )
        .await
    }

    pub async fn paginate(
        &mut self,
        room_id: &str,
        direction: NativeTimelineDirection,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let entry = self
            .entries
            .get_mut(&room_id)
            .ok_or("d0.3-timeline-not-open")?;
        let reached_end = match direction {
            NativeTimelineDirection::Backwards => entry
                .timeline
                .paginate_backwards(PAGINATION_BATCH_SIZE)
                .await
                .map_err(|_| "d0.3-timeline-paginate-backwards-failed")?,
            NativeTimelineDirection::Forwards => entry
                .timeline
                .paginate_forwards(PAGINATION_BATCH_SIZE)
                .await
                .map_err(|_| "d0.3-timeline-paginate-forwards-failed")?,
        };
        if direction == NativeTimelineDirection::Backwards {
            entry.hit_start = reached_end;
        }
        snapshot_from_timeline(
            self.session_generation,
            room_id,
            &entry.timeline,
            entry.hit_start,
        )
        .await
    }
}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| "d0.3-timeline-invalid-room-id")
}

async fn snapshot_from_timeline(
    session_generation: u64,
    room_id: String,
    timeline: &Timeline,
    hit_start: bool,
) -> Result<NativeTimelineSnapshot, &'static str> {
    let (items, _updates) = timeline.subscribe().await;
    let items = items.iter().filter_map(|item| project_item(item)).collect();
    Ok(NativeTimelineSnapshot {
        session_generation,
        room_id,
        items,
        hit_start,
    })
}

fn project_item(item: &SdkTimelineItem) -> Option<NativeTimelineItem> {
    let event = item.as_event()?;
    let event_id = event.event_id()?.to_string();
    let content = event.content();
    Some(NativeTimelineItem {
        item_id: item.unique_id().0.clone(),
        event_id,
        sender: event.sender().to_string(),
        event_type: safe_event_type(content),
        body: safe_body(content),
        origin_server_ts: event.timestamp().get().into(),
    })
}

fn safe_event_type(content: &SdkTimelineItemContent) -> String {
    if content.is_redacted() {
        return "m.room.redacted".to_owned();
    }
    content
        .event_type_str()
        .unwrap_or_else(|| "m.room.unknown".to_owned())
}

fn safe_body(content: &SdkTimelineItemContent) -> String {
    safe_body_from_parts(
        content.is_redacted(),
        content.is_unable_to_decrypt(),
        content.as_message().map(|message| message.body()),
    )
}

fn safe_body_from_parts(redacted: bool, unable_to_decrypt: bool, body: Option<&str>) -> String {
    if redacted {
        REDACTED_PLACEHOLDER.to_owned()
    } else if unable_to_decrypt {
        UTD_PLACEHOLDER.to_owned()
    } else if let Some(body) = body {
        body.to_owned()
    } else {
        UNSUPPORTED_PLACEHOLDER.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_snapshot_schema_has_no_secret_or_ciphertext_fields() {
        let snapshot = NativeTimelineSnapshot {
            session_generation: 7,
            room_id: "!room:example.org".into(),
            items: vec![NativeTimelineItem {
                item_id: "item-1".into(),
                event_id: "$event".into(),
                sender: "@alice:example.org".into(),
                event_type: "m.room.message".into(),
                body: "hello".into(),
                origin_server_ts: 42,
            }],
            hit_start: false,
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        for forbidden in [
            "accessToken",
            "access_token",
            "refreshToken",
            "refresh_token",
            "sessionKey",
            "ciphertext",
        ] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("\"type\":\"m.room.message\""));
        assert!(json.contains("\"body\":\"hello\""));
    }

    #[test]
    fn invalid_room_ids_are_rejected_before_sdk_lookup() {
        assert_eq!(
            parse_room_id("not-a-room").unwrap_err(),
            "d0.3-timeline-invalid-room-id"
        );
    }

    #[test]
    fn safe_body_projection_never_exposes_unavailable_event_content() {
        assert_eq!(
            safe_body_from_parts(true, false, Some("ignored")),
            REDACTED_PLACEHOLDER
        );
        assert_eq!(
            safe_body_from_parts(false, true, Some("ignored")),
            UTD_PLACEHOLDER
        );
        assert_eq!(
            safe_body_from_parts(false, false, None),
            UNSUPPORTED_PLACEHOLDER
        );
        assert_eq!(
            safe_body_from_parts(false, false, Some("clear text")),
            "clear text"
        );
    }
}
