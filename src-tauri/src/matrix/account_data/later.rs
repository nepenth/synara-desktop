//! Live `in.synara.later` Client RMW. Codec types live in synara-core.

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
    },
    Client,
};
use serde_json::value::to_raw_value;

pub use synara_core::app::account_data::{
    clear_completed_later_items, complete_later_item, mark_later_reminded, normalize_later_content,
    normalize_later_item, put_later_item, snooze_later_item, NativeLaterSnapshot,
    SynaraLaterContent, SynaraLaterItem, SynaraLaterItemKind, LATER_ACCOUNT_DATA_VERSION,
    LATER_EVENT_TYPE,
};

fn later_event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(LATER_EVENT_TYPE)
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
