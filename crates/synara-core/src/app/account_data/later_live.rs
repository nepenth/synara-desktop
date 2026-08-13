//! Live `in.synara.later` Client RMW owned by the shared native core.

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
    },
    Client,
};
use serde_json::value::to_raw_value;

use super::{
    clear_completed_later_items, complete_later_item, mark_later_reminded, normalize_later_content,
    put_later_item, snooze_later_item, NativeLaterSnapshot, SynaraLaterContent, SynaraLaterItem,
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

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

pub fn later_timestamp_or_now(value: Option<f64>) -> f64 {
    value.filter(|v| v.is_finite()).unwrap_or_else(now_ms)
}
