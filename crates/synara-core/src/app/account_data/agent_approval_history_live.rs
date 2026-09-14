//! Live `in.synara.agent_approval_history` Client RMW owned by the shared native core.

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
    },
    Client,
};
use serde::Deserialize;
use serde_json::value::to_raw_value;
use serde_json::value::RawValue as RawJsonValue;

use super::{
    append_agent_approval_history_item, normalize_agent_approval_history_content_checked,
    validate_agent_approval_history_content_size, validate_agent_approval_history_item,
    NativeAgentApprovalHistorySnapshot, SynaraAgentApprovalHistoryContent,
    SynaraAgentApprovalHistoryItem, AGENT_APPROVAL_HISTORY_EVENT_TYPE,
    MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES,
};

fn agent_approval_history_event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(AGENT_APPROVAL_HISTORY_EVENT_TYPE)
}

pub(super) fn parse_agent_approval_history_content(
    raw: Option<Raw<AnyGlobalAccountDataEventContent>>,
    now_ms: f64,
) -> Result<SynaraAgentApprovalHistoryContent, &'static str> {
    let Some(raw) = raw else {
        return Ok(SynaraAgentApprovalHistoryContent::default());
    };
    if raw.json().get().len() > MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES {
        return Err("agent-approval-history-payload-too-large");
    }
    let value = raw
        .deserialize_as_unchecked::<serde_json::Value>()
        .map_err(|_| "agent-approval-history-deserialize-failed")?;
    normalize_agent_approval_history_content_checked(Some(&value), now_ms)
}

#[derive(Deserialize)]
struct RawAgentApprovalHistorySyncEvent {
    content: Box<RawJsonValue>,
}

pub(super) fn parse_agent_approval_history_sync_event(
    raw_event: &RawJsonValue,
    now_ms: f64,
) -> Result<SynaraAgentApprovalHistoryContent, &'static str> {
    const MAX_SYNC_EVENT_OVERHEAD_BYTES: usize = 256;
    if raw_event.get().len()
        > MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES.saturating_add(MAX_SYNC_EVENT_OVERHEAD_BYTES)
    {
        return Err("agent-approval-history-payload-too-large");
    }
    let event: RawAgentApprovalHistorySyncEvent = serde_json::from_str(raw_event.get())
        .map_err(|_| "agent-approval-history-deserialize-failed")?;
    parse_agent_approval_history_content(Some(Raw::from_json(event.content)), now_ms)
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

async fn load_cached_agent_approval_history_content(
    client: &Client,
) -> Result<SynaraAgentApprovalHistoryContent, &'static str> {
    let raw = client
        .account()
        .account_data_raw(agent_approval_history_event_type())
        .await
        .map_err(|_| "agent-approval-history-load-failed")?;
    parse_agent_approval_history_content(raw, now_ms())
}

async fn fetch_fresh_agent_approval_history_content(
    client: &Client,
) -> Result<SynaraAgentApprovalHistoryContent, &'static str> {
    let raw = client
        .account()
        // A successful set_account_data_raw does not update the SDK state
        // store. Fetch from the homeserver for every serialized RMW so an
        // immediate second mutation cannot reload a pre-write /sync snapshot.
        .fetch_account_data(agent_approval_history_event_type())
        .await
        .map_err(|_| "agent-approval-history-fetch-failed")?;
    parse_agent_approval_history_content(raw, now_ms())
}

async fn store_agent_approval_history_content(
    client: &Client,
    content: &SynaraAgentApprovalHistoryContent,
) -> Result<(), &'static str> {
    validate_agent_approval_history_content_size(content)?;
    let raw_value = to_raw_value(content).map_err(|_| "agent-approval-history-serialize-failed")?;
    let raw = Raw::<AnyGlobalAccountDataEventContent>::from_json(raw_value);
    client
        .account()
        .set_account_data_raw(agent_approval_history_event_type(), raw)
        .await
        .map_err(|_| "agent-approval-history-set-failed")?;
    Ok(())
}

pub async fn snapshot_agent_approval_history(
    client: &Client,
) -> Result<NativeAgentApprovalHistorySnapshot, &'static str> {
    Ok(NativeAgentApprovalHistorySnapshot {
        items: load_cached_agent_approval_history_content(client)
            .await?
            .items,
    })
}

pub async fn append_agent_approval_history_item_live(
    client: &Client,
    item: SynaraAgentApprovalHistoryItem,
) -> Result<NativeAgentApprovalHistorySnapshot, &'static str> {
    validate_agent_approval_history_item(&item)?;
    let now = now_ms();
    let next = append_agent_approval_history_item(
        fetch_fresh_agent_approval_history_content(client).await?,
        item,
        now,
    );
    store_agent_approval_history_content(client, &next).await?;
    Ok(NativeAgentApprovalHistorySnapshot { items: next.items })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_account_data(json: String) -> Raw<AnyGlobalAccountDataEventContent> {
        Raw::from_json(serde_json::value::RawValue::from_string(json).expect("valid raw JSON"))
    }

    #[test]
    fn rejects_oversized_raw_whitespace_before_deserializing() {
        let payload = format!(
            "{{\"version\":1,{}\"items\":[]}}",
            " ".repeat(MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES)
        );
        assert_eq!(
            parse_agent_approval_history_content(Some(raw_account_data(payload)), 1.0),
            Err("agent-approval-history-payload-too-large")
        );
    }

    #[test]
    fn synchronized_event_content_is_bounded_and_version_checked() {
        let supported = serde_json::value::RawValue::from_string(
            r#"{"type":"in.synara.agent_approval_history","content":{"version":1,"items":[]}}"#
                .to_owned(),
        )
        .expect("valid sync event");
        assert_eq!(
            parse_agent_approval_history_sync_event(&supported, 1.0),
            Ok(SynaraAgentApprovalHistoryContent::default())
        );

        let unknown = serde_json::value::RawValue::from_string(
            r#"{"type":"in.synara.agent_approval_history","content":{"version":2,"items":[]}}"#
                .to_owned(),
        )
        .expect("valid sync event");
        assert_eq!(
            parse_agent_approval_history_sync_event(&unknown, 1.0),
            Err("agent-approval-history-unsupported-version")
        );
    }
}
