//! Native `in.synara.later` account-data codec and live RMW owner.

use std::collections::BTreeMap;

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::value::to_raw_value;

pub const LATER_EVENT_TYPE: &str = "in.synara.later";
pub const LATER_ACCOUNT_DATA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynaraLaterContent {
    pub version: u32,
    pub items: BTreeMap<String, SynaraLaterItem>,
}

impl Default for SynaraLaterContent {
    fn default() -> Self {
        Self {
            version: LATER_ACCOUNT_DATA_VERSION,
            items: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynaraLaterItem {
    pub id: String,
    pub kind: SynaraLaterItemKind,
    pub room_id: String,
    pub event_id: String,
    pub created_at: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_ts: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminded_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SynaraLaterItemKind {
    Saved,
    Reminder,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLaterSnapshot {
    pub session_generation: u64,
    pub content: SynaraLaterContent,
}

fn later_event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(LATER_EVENT_TYPE)
}

fn finite_ts(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite())
}

pub fn normalize_later_item(item: &serde_json::Value) -> Option<SynaraLaterItem> {
    let id = item.get("id")?.as_str()?.to_owned();
    let kind = match item.get("kind")?.as_str()? {
        "saved" => SynaraLaterItemKind::Saved,
        "reminder" => SynaraLaterItemKind::Reminder,
        _ => return None,
    };
    let room_id = item.get("roomId")?.as_str()?.to_owned();
    let event_id = item.get("eventId")?.as_str()?.to_owned();
    let created_at = item.get("createdAt")?.as_f64()?;
    if !created_at.is_finite() || id.is_empty() || room_id.is_empty() || event_id.is_empty() {
        return None;
    }
    Some(SynaraLaterItem {
        id,
        kind,
        room_id,
        event_id,
        created_at,
        due_ts: finite_ts(item.get("dueTs").and_then(|v| v.as_f64())),
        reminded_at: finite_ts(item.get("remindedAt").and_then(|v| v.as_f64())),
        completed_at: finite_ts(item.get("completedAt").and_then(|v| v.as_f64())),
    })
}

pub fn normalize_later_content(value: Option<&serde_json::Value>) -> SynaraLaterContent {
    let mut items = BTreeMap::new();
    if let Some(raw_items) = value
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_object())
    {
        for (item_id, item) in raw_items {
            if let Some(normalized) = normalize_later_item(item) {
                items.insert(item_id.clone(), normalized);
            }
        }
    }
    SynaraLaterContent {
        version: LATER_ACCOUNT_DATA_VERSION,
        items,
    }
}

pub fn put_later_item(content: SynaraLaterContent, item: SynaraLaterItem) -> SynaraLaterContent {
    let mut next = content;
    next.version = LATER_ACCOUNT_DATA_VERSION;
    next.items.insert(item.id.clone(), item);
    next
}

pub fn complete_later_item(
    content: SynaraLaterContent,
    item_id: &str,
    completed_at: f64,
) -> SynaraLaterContent {
    let mut next = content;
    if let Some(item) = next.items.get_mut(item_id) {
        item.completed_at = Some(completed_at);
    }
    next
}

pub fn snooze_later_item(
    content: SynaraLaterContent,
    item_id: &str,
    due_ts: f64,
) -> SynaraLaterContent {
    let mut next = content;
    if let Some(item) = next.items.get_mut(item_id) {
        item.kind = SynaraLaterItemKind::Reminder;
        item.due_ts = Some(due_ts);
        item.reminded_at = None;
        item.completed_at = None;
    }
    next
}

pub fn clear_completed_later_items(content: SynaraLaterContent) -> SynaraLaterContent {
    let mut next = content;
    next.items.retain(|_, item| item.completed_at.is_none());
    next
}

pub fn mark_later_reminded(
    content: SynaraLaterContent,
    item_id: &str,
    reminded_at: f64,
) -> SynaraLaterContent {
    let mut next = content;
    if let Some(item) = next.items.get_mut(item_id) {
        item.reminded_at = Some(reminded_at);
    }
    next
}

async fn load_later_content(client: &Client) -> Result<SynaraLaterContent, &'static str> {
    let raw = client
        .account()
        .account_data_raw(later_event_type())
        .await
        .map_err(|_| "v-timeline-later-fetch-failed")?;
    let value = match raw {
        Some(raw) => raw
            .deserialize_as_unchecked::<serde_json::Value>()
            .map_err(|_| "v-timeline-later-deserialize-failed")?,
        None => return Ok(SynaraLaterContent::default()),
    };
    Ok(normalize_later_content(Some(&value)))
}

async fn store_later_content(
    client: &Client,
    content: &SynaraLaterContent,
) -> Result<(), &'static str> {
    let raw_value = to_raw_value(content).map_err(|_| "v-timeline-later-serialize-failed")?;
    let raw = Raw::<AnyGlobalAccountDataEventContent>::from_json(raw_value);
    client
        .account()
        .set_account_data_raw(later_event_type(), raw)
        .await
        .map_err(|_| "v-timeline-later-set-failed")?;
    Ok(())
}

pub async fn snapshot_later(
    client: &Client,
    session_generation: u64,
) -> Result<NativeLaterSnapshot, &'static str> {
    Ok(NativeLaterSnapshot {
        session_generation,
        content: load_later_content(client).await?,
    })
}

async fn mutate_later<F>(
    client: &Client,
    session_generation: u64,
    mutate: F,
) -> Result<NativeLaterSnapshot, &'static str>
where
    F: FnOnce(SynaraLaterContent) -> SynaraLaterContent,
{
    let next = mutate(load_later_content(client).await?);
    store_later_content(client, &next).await?;
    Ok(NativeLaterSnapshot {
        session_generation,
        content: next,
    })
}

pub async fn upsert_later_item(
    client: &Client,
    session_generation: u64,
    item: SynaraLaterItem,
) -> Result<NativeLaterSnapshot, &'static str> {
    if item.id.is_empty() || item.room_id.is_empty() || item.event_id.is_empty() {
        return Err("v-timeline-later-invalid-item");
    }
    if !item.created_at.is_finite() {
        return Err("v-timeline-later-invalid-item");
    }
    mutate_later(client, session_generation, |content| {
        put_later_item(content, item)
    })
    .await
}

pub async fn complete_later_item_live(
    client: &Client,
    session_generation: u64,
    item_id: String,
    completed_at: f64,
) -> Result<NativeLaterSnapshot, &'static str> {
    if item_id.is_empty() || !completed_at.is_finite() {
        return Err("v-timeline-later-invalid-item");
    }
    mutate_later(client, session_generation, |content| {
        complete_later_item(content, &item_id, completed_at)
    })
    .await
}

pub async fn snooze_later_item_live(
    client: &Client,
    session_generation: u64,
    item_id: String,
    due_ts: f64,
) -> Result<NativeLaterSnapshot, &'static str> {
    if item_id.is_empty() || !due_ts.is_finite() {
        return Err("v-timeline-later-invalid-item");
    }
    mutate_later(client, session_generation, |content| {
        snooze_later_item(content, &item_id, due_ts)
    })
    .await
}

pub async fn clear_completed_later_live(
    client: &Client,
    session_generation: u64,
) -> Result<NativeLaterSnapshot, &'static str> {
    mutate_later(client, session_generation, clear_completed_later_items).await
}

pub async fn mark_later_reminded_live(
    client: &Client,
    session_generation: u64,
    item_id: String,
    reminded_at: f64,
) -> Result<NativeLaterSnapshot, &'static str> {
    if item_id.is_empty() || !reminded_at.is_finite() {
        return Err("v-timeline-later-invalid-item");
    }
    mutate_later(client, session_generation, |content| {
        mark_later_reminded(content, &item_id, reminded_at)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_drops_invalid_and_keeps_privacy_fields_only() {
        let value = json!({
            "version": 1,
            "items": {
                "good": {
                    "id": "!r:s\n$e",
                    "kind": "saved",
                    "roomId": "!r:s",
                    "eventId": "$e",
                    "createdAt": 10.0,
                    "body": "must-not-persist"
                },
                "bad": { "id": 1 }
            }
        });
        let content = normalize_later_content(Some(&value));
        assert_eq!(content.version, 1);
        assert_eq!(content.items.len(), 1);
        let item = content.items.get("good").expect("item");
        assert_eq!(item.kind, SynaraLaterItemKind::Saved);
        let serialized = serde_json::to_value(item).expect("serialize");
        assert!(serialized.get("body").is_none());
    }

    #[test]
    fn mutate_helpers_match_js_semantics() {
        let item = SynaraLaterItem {
            id: "!r:s\n$e".into(),
            kind: SynaraLaterItemKind::Saved,
            room_id: "!r:s".into(),
            event_id: "$e".into(),
            created_at: 1.0,
            due_ts: None,
            reminded_at: None,
            completed_at: None,
        };
        let content = put_later_item(SynaraLaterContent::default(), item);
        let snoozed = snooze_later_item(content.clone(), "!r:s\n$e", 99.0);
        assert_eq!(
            snoozed.items["!r:s\n$e"].kind,
            SynaraLaterItemKind::Reminder
        );
        assert_eq!(snoozed.items["!r:s\n$e"].due_ts, Some(99.0));
        let completed = complete_later_item(snoozed, "!r:s\n$e", 100.0);
        assert_eq!(completed.items["!r:s\n$e"].completed_at, Some(100.0));
        let cleared = clear_completed_later_items(completed);
        assert!(cleared.items.is_empty());
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = NativeLaterSnapshot {
            session_generation: 3,
            content: SynaraLaterContent::default(),
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 3);
        assert_eq!(value["content"]["version"], 1);
        assert_eq!(value["content"]["items"], json!({}));
    }
}
