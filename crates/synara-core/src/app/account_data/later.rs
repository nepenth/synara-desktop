//! Credential-free `in.synara.later` account-data codec.
//!
//! Live Client RMW stays in the desktop shell.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLaterSnapshot {
    pub session_generation: u64,
    pub content: SynaraLaterContent,
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
