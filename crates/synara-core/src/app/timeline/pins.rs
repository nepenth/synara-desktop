//! Native pin panel backed by `EventCache::pinned_events` / `PinnedEventsCache`.
//!
//! Live timeline `pinnedEventIds` stay on `room.pinned_event_ids()` for the
//! row ⋯ menu. The pin **list** projects cache events (body, media handle,
//! reactions with recovered ids) so unpin of a non-final pin updates without
//! a JS `useRoomEvent` refetch.

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use matrix_sdk::ruma::events::{AnySyncMessageLikeEvent, AnySyncTimelineEvent};
use matrix_sdk::Room;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use super::media::TimelineMediaRegistry;
use super::reactions::{
    annotation_from_timeline_event, fill_sender_event_id, recover_missing_annotation_ids,
    RecoveredAnnotation,
};
use super::view::{
    project_message_type_and_media, TimelineMediaHandle, TimelineReaction, TimelineReactionSender,
};
use crate::app::room_list::last_message_preview_from_event_json_str;

pub const PINNED_EVENTS_SCHEMA_VERSION: u32 = 1;

/// Exact React/Tauri envelope payload for `matrix_pinned_events`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePinnedEventsRequest {
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedEventItem {
    pub event_id: String,
    pub sender_id: String,
    pub origin_server_ts: u64,
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub redacted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<TimelineReaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<TimelineMediaHandle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedEventsSnapshot {
    pub schema_version: u32,
    pub room_id: String,
    pub event_ids: Vec<String>,
    pub items: Vec<PinnedEventItem>,
}

/// Load the current `PinnedEventsCache` for `room` and project native pin rows.
pub async fn snapshot_pinned_events(
    room: &Room,
    media: &mut TimelineMediaRegistry,
) -> Result<PinnedEventsSnapshot, &'static str> {
    room.client()
        .event_cache()
        .subscribe()
        .map_err(|_| "v-timeline-pinned-cache-unavailable")?;
    let (cache, _handles) = room
        .client()
        .event_cache()
        .pinned_events(room.room_id())
        .await
        .map_err(|_| "v-timeline-pinned-cache-unavailable")?;
    let event_ids: Vec<String> = room
        .pinned_event_ids()
        .unwrap_or_default()
        .into_iter()
        .map(|event_id| event_id.to_string())
        .collect();
    let events = wait_for_pinned_cache_events(&cache, &event_ids).await?;
    let own_user_id = room.client().user_id().map(|user_id| user_id.to_string());
    let mut items = project_pinned_items(&event_ids, &events, own_user_id.as_deref(), media);
    let targets: HashSet<String> = event_ids.iter().cloned().collect();
    if !targets.is_empty() {
        let recovered = recover_missing_annotation_ids(room, &targets, true).await;
        for item in &mut items {
            for reaction in &mut item.reactions {
                for sender in &mut reaction.senders {
                    fill_sender_event_id(
                        &item.event_id,
                        &reaction.key,
                        &sender.user_id,
                        &mut sender.reaction_event_id,
                        &recovered,
                    );
                }
            }
        }
    }
    Ok(PinnedEventsSnapshot {
        schema_version: PINNED_EVENTS_SCHEMA_VERSION,
        room_id: room.room_id().to_string(),
        event_ids,
        items,
    })
}

async fn wait_for_pinned_cache_events(
    cache: &matrix_sdk::event_cache::PinnedEventsCache,
    event_ids: &[String],
) -> Result<Vec<matrix_sdk::deserialized_responses::TimelineEvent>, &'static str> {
    let wanted: HashSet<&str> = event_ids.iter().map(String::as_str).collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let deadline = timeout(Duration::from_secs(4), async {
        loop {
            let (events, _updates) = cache
                .subscribe()
                .await
                .map_err(|_| "v-timeline-pinned-subscribe-failed")?;
            if events.iter().any(|event| {
                event
                    .event_id()
                    .is_some_and(|event_id| wanted.contains(event_id.as_str()))
            }) {
                return Ok(events);
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    })
    .await;
    match deadline {
        Ok(result) => result,
        Err(_) => cache
            .subscribe()
            .await
            .map(|(events, _)| events)
            .map_err(|_| "v-timeline-pinned-subscribe-failed"),
    }
}

pub fn project_pinned_items(
    pinned_ids: &[String],
    events: &[matrix_sdk::deserialized_responses::TimelineEvent],
    own_user_id: Option<&str>,
    media: &mut TimelineMediaRegistry,
) -> Vec<PinnedEventItem> {
    let annotations: Vec<RecoveredAnnotation> = events
        .iter()
        .filter_map(annotation_from_timeline_event)
        .collect();
    let by_id: BTreeMap<String, &matrix_sdk::deserialized_responses::TimelineEvent> = events
        .iter()
        .filter_map(|event| Some((event.event_id()?.to_string(), event)))
        .collect();
    pinned_ids
        .iter()
        .filter_map(|event_id| {
            let event = *by_id.get(event_id)?;
            Some(project_one_pinned_item(
                event_id,
                event,
                &annotations,
                own_user_id,
                media,
            ))
        })
        .collect()
}

fn project_one_pinned_item(
    event_id: &str,
    event: &matrix_sdk::deserialized_responses::TimelineEvent,
    annotations: &[RecoveredAnnotation],
    own_user_id: Option<&str>,
    media: &mut TimelineMediaRegistry,
) -> PinnedEventItem {
    let deserialized = event.raw().deserialize().ok();
    let (event_type, redacted, message_type, media_handle) = match deserialized.as_ref() {
        Some(AnySyncTimelineEvent::MessageLike(message)) => {
            let event_type = message.event_type().to_string();
            match message {
                AnySyncMessageLikeEvent::RoomMessage(message) => match message.as_original() {
                    Some(original) => {
                        let (message_type, media_handle) = project_message_type_and_media(
                            event_id,
                            &original.content.msgtype,
                            Some(media),
                        );
                        (event_type, false, message_type, media_handle)
                    }
                    None => (event_type, true, None, None),
                },
                AnySyncMessageLikeEvent::Sticker(sticker) => (
                    event_type,
                    sticker.as_original().is_none(),
                    Some("sticker".into()),
                    None,
                ),
                other => (other.event_type().to_string(), false, None, None),
            }
        }
        Some(other) => (other.event_type().to_string(), false, None, None),
        None => ("unknown".into(), false, None, None),
    };
    PinnedEventItem {
        event_id: event_id.to_owned(),
        sender_id: event
            .sender()
            .map(|sender| sender.to_string())
            .unwrap_or_default(),
        origin_server_ts: event.timestamp().map(|ts| u64::from(ts.get())).unwrap_or(0),
        event_type,
        body: last_message_preview_from_event_json_str(event.raw().json().get()),
        redacted,
        reactions: reactions_for_target(event_id, annotations, own_user_id),
        media: media_handle,
        message_type,
    }
}

pub fn reactions_for_target(
    target_event_id: &str,
    annotations: &[RecoveredAnnotation],
    own_user_id: Option<&str>,
) -> Vec<TimelineReaction> {
    let mut by_key: BTreeMap<String, Vec<TimelineReactionSender>> = BTreeMap::new();
    for annotation in annotations
        .iter()
        .filter(|annotation| annotation.target_event_id == target_event_id)
    {
        by_key
            .entry(annotation.key.clone())
            .or_default()
            .push(TimelineReactionSender {
                user_id: annotation.sender.clone(),
                reaction_event_id: Some(annotation.reaction_event_id.clone()),
            });
    }
    by_key
        .into_iter()
        .map(|(key, senders)| {
            let own = own_user_id.map(|own| senders.iter().any(|sender| sender.user_id == own));
            TimelineReaction {
                key,
                count: u32::try_from(senders.len()).unwrap_or(u32::MAX),
                own,
                senders,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_panel_source_uses_pinned_events_cache() {
        let source = include_str!("pins.rs");
        assert!(source.contains("pinned_events("));
        assert!(source.contains("PinnedEventsCache"));
        assert!(source.contains("recover_missing_annotation_ids"));
        assert!(!source.contains("TimelineFocus::PinnedEvents"));
    }

    #[test]
    fn reactions_for_target_keeps_annotation_ids() {
        let annotations = vec![
            RecoveredAnnotation {
                reaction_event_id: "$bob-react".into(),
                sender: "@bob:example.org".into(),
                target_event_id: "$pinned".into(),
                key: "👍".into(),
            },
            RecoveredAnnotation {
                reaction_event_id: "$me-react".into(),
                sender: "@alice:example.org".into(),
                target_event_id: "$pinned".into(),
                key: "👍".into(),
            },
        ];
        let reactions = reactions_for_target("$pinned", &annotations, Some("@alice:example.org"));
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].count, 2);
        assert_eq!(reactions[0].own, Some(true));
        assert_eq!(
            reactions[0].senders[0].reaction_event_id.as_deref(),
            Some("$bob-react")
        );
        assert!(reactions_for_target("$other", &annotations, None).is_empty());
    }
}
