//! Credential-free `in.synara.agent_approval_history` account-data codec.
//!
//! Live Client RMW is in `agent_approval_history_live`. Account data is
//! server-readable plaintext even for encrypted rooms, so items store a
//! bounded command-preview summary and never the full command body.

use serde::{Deserialize, Serialize};

use matrix_sdk::ruma::OwnedServerName;

use crate::app::agent_approvals::{
    sanitize_agent_approval_history_summary, AGENT_APPROVAL_ACTION_APPROVE_ALWAYS,
    AGENT_APPROVAL_ACTION_APPROVE_ONCE, AGENT_APPROVAL_ACTION_DENY,
};

pub const AGENT_APPROVAL_HISTORY_EVENT_TYPE: &str = "in.synara.agent_approval_history";
pub const AGENT_APPROVAL_HISTORY_UPDATED_EVENT: &str = "matrix-agent-approval-history-updated";
pub const AGENT_APPROVAL_HISTORY_ACCOUNT_DATA_VERSION: u32 = 1;
pub const MAX_AGENT_APPROVAL_HISTORY_ITEMS: usize = 200;
pub const MAX_AGENT_APPROVAL_HISTORY_SUMMARY_CHARS: usize = 240;
pub const MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES: usize = 262_144;
pub const AGENT_APPROVAL_HISTORY_RETENTION_MS: f64 = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;
pub const MAX_AGENT_APPROVAL_HISTORY_ROOM_ID_BYTES: usize = 255;
pub const MAX_AGENT_APPROVAL_HISTORY_EVENT_ID_BYTES: usize = 255;
pub const MAX_AGENT_APPROVAL_HISTORY_SENDER_LENGTH: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynaraAgentApprovalHistoryDecision {
    ApproveOnce,
    ApproveAlways,
    Deny,
}

impl SynaraAgentApprovalHistoryDecision {
    pub fn from_action_id(action_id: &str) -> Option<Self> {
        match action_id {
            AGENT_APPROVAL_ACTION_APPROVE_ONCE => Some(Self::ApproveOnce),
            AGENT_APPROVAL_ACTION_APPROVE_ALWAYS => Some(Self::ApproveAlways),
            AGENT_APPROVAL_ACTION_DENY => Some(Self::Deny),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynaraAgentApprovalHistoryItem {
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub decision: SynaraAgentApprovalHistoryDecision,
    pub decided_at: f64,
    pub origin_server_ts: f64,
    pub expires_at: f64,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynaraAgentApprovalHistoryContent {
    pub version: u32,
    pub items: Vec<SynaraAgentApprovalHistoryItem>,
}

impl Default for SynaraAgentApprovalHistoryContent {
    fn default() -> Self {
        Self {
            version: AGENT_APPROVAL_HISTORY_ACCOUNT_DATA_VERSION,
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAgentApprovalHistorySnapshot {
    pub items: Vec<SynaraAgentApprovalHistoryItem>,
}

fn has_disallowed_id_char(ch: char) -> bool {
    ch.is_whitespace()
        || ch.is_control()
        || matches!(
            ch,
            '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
}

fn is_matrix_room_id(value: &str) -> bool {
    if !value.starts_with('!')
        || value.len() <= 1
        || value.len() > MAX_AGENT_APPROVAL_HISTORY_ROOM_ID_BYTES
        || value.chars().any(has_disallowed_id_char)
    {
        return false;
    }
    let Some((_, server)) = value.split_once(':') else {
        return false;
    };
    OwnedServerName::try_from(server).is_ok()
}

fn is_matrix_event_id(value: &str) -> bool {
    value.starts_with('$')
        && value.len() > 1
        && value.len() <= MAX_AGENT_APPROVAL_HISTORY_EVENT_ID_BYTES
        && !value.chars().any(has_disallowed_id_char)
}

fn is_matrix_user_id(value: &str) -> bool {
    if value.chars().count() > MAX_AGENT_APPROVAL_HISTORY_SENDER_LENGTH
        || value.chars().any(has_disallowed_id_char)
    {
        return false;
    }
    let Some((local, server)) = value.split_once(':') else {
        return false;
    };
    local.starts_with('@') && local.len() > 1 && OwnedServerName::try_from(server).is_ok()
}

fn limit_summary(value: &str) -> String {
    sanitize_agent_approval_history_summary(value)
}

fn finite_ts(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite())
}

pub fn normalize_agent_approval_history_item(
    item: &serde_json::Value,
) -> Option<SynaraAgentApprovalHistoryItem> {
    let room_id = item.get("roomId")?.as_str()?.to_owned();
    let event_id = item.get("eventId")?.as_str()?.to_owned();
    let sender = item.get("sender")?.as_str()?.to_owned();
    let decision = match item.get("decision")?.as_str()? {
        "approve_once" => SynaraAgentApprovalHistoryDecision::ApproveOnce,
        "approve_always" => SynaraAgentApprovalHistoryDecision::ApproveAlways,
        "deny" => SynaraAgentApprovalHistoryDecision::Deny,
        _ => return None,
    };
    let decided_at = finite_ts(item.get("decidedAt").and_then(|v| v.as_f64()))?;
    let origin_server_ts = finite_ts(item.get("originServerTs").and_then(|v| v.as_f64()))?;
    let expires_at = finite_ts(item.get("expiresAt").and_then(|v| v.as_f64()))?;
    if !is_matrix_room_id(&room_id)
        || !is_matrix_event_id(&event_id)
        || !is_matrix_user_id(&sender)
        || decided_at <= 0.0
        || origin_server_ts <= 0.0
        || expires_at <= origin_server_ts
    {
        return None;
    }
    let summary = item
        .get("summary")
        .and_then(|v| v.as_str())
        .map(limit_summary)
        .unwrap_or_default();
    Some(SynaraAgentApprovalHistoryItem {
        room_id,
        event_id,
        sender,
        decision,
        decided_at,
        origin_server_ts,
        expires_at,
        summary,
    })
}

/// Parses live account data without silently rewriting a schema owned by a
/// newer client. Mutations must use this checked form so an unsupported
/// version cannot be normalized to v1 and written back destructively.
pub fn normalize_agent_approval_history_content_checked(
    value: Option<&serde_json::Value>,
    now_ms: f64,
) -> Result<SynaraAgentApprovalHistoryContent, &'static str> {
    if let Some(value) = value {
        let encoded_size = serde_json::to_vec(value)
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX);
        if encoded_size > MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES {
            return Err("agent-approval-history-payload-too-large");
        }
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or("agent-approval-history-unsupported-version")?;
        if version != u64::from(AGENT_APPROVAL_HISTORY_ACCOUNT_DATA_VERSION) {
            return Err("agent-approval-history-unsupported-version");
        }
        if let Some(items) = value.get("items") {
            if !items.is_array() {
                return Err("agent-approval-history-invalid-items");
            }
        }
    }
    Ok(normalize_agent_approval_history_content(value, now_ms))
}

pub fn validate_agent_approval_history_content_size(
    content: &SynaraAgentApprovalHistoryContent,
) -> Result<(), &'static str> {
    let encoded_size = serde_json::to_vec(content)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if encoded_size > MAX_AGENT_APPROVAL_HISTORY_CONTENT_BYTES {
        return Err("agent-approval-history-payload-too-large");
    }
    Ok(())
}

pub fn validate_agent_approval_history_item(
    item: &SynaraAgentApprovalHistoryItem,
) -> Result<(), &'static str> {
    if !is_matrix_room_id(&item.room_id)
        || !is_matrix_event_id(&item.event_id)
        || !is_matrix_user_id(&item.sender)
        || !item.decided_at.is_finite()
        || !item.origin_server_ts.is_finite()
        || !item.expires_at.is_finite()
        || item.decided_at <= 0.0
        || item.origin_server_ts <= 0.0
        || item.expires_at <= item.origin_server_ts
        || item.summary.chars().count() > MAX_AGENT_APPROVAL_HISTORY_SUMMARY_CHARS
        || item.summary.chars().any(|ch| {
            ch.is_control()
                || matches!(
                    ch,
                    '\u{200B}'..='\u{200F}'
                        | '\u{202A}'..='\u{202E}'
                        | '\u{2066}'..='\u{2069}'
                        | '\u{FEFF}'
                )
        })
    {
        return Err("agent-approval-history-invalid-item");
    }
    Ok(())
}

pub fn prune_agent_approval_history_items(
    items: Vec<SynaraAgentApprovalHistoryItem>,
    now_ms: f64,
) -> Vec<SynaraAgentApprovalHistoryItem> {
    let cutoff = if now_ms.is_finite() {
        now_ms - AGENT_APPROVAL_HISTORY_RETENTION_MS
    } else {
        f64::NEG_INFINITY
    };
    let mut pruned: Vec<SynaraAgentApprovalHistoryItem> = Vec::new();
    for item in items {
        if item.decided_at < cutoff {
            continue;
        }
        if let Some(existing) = pruned.iter_mut().find(|candidate| {
            candidate.room_id == item.room_id && candidate.event_id == item.event_id
        }) {
            if item.decided_at >= existing.decided_at {
                *existing = item;
            }
            continue;
        }
        pruned.push(item);
    }
    pruned.sort_by(|a, b| {
        b.decided_at
            .partial_cmp(&a.decided_at)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    pruned.truncate(MAX_AGENT_APPROVAL_HISTORY_ITEMS);
    pruned
}

pub fn normalize_agent_approval_history_content(
    value: Option<&serde_json::Value>,
    now_ms: f64,
) -> SynaraAgentApprovalHistoryContent {
    let items = value
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(normalize_agent_approval_history_item)
                .collect()
        })
        .unwrap_or_default();
    SynaraAgentApprovalHistoryContent {
        version: AGENT_APPROVAL_HISTORY_ACCOUNT_DATA_VERSION,
        items: prune_agent_approval_history_items(items, now_ms),
    }
}

pub fn append_agent_approval_history_item(
    content: SynaraAgentApprovalHistoryContent,
    item: SynaraAgentApprovalHistoryItem,
    now_ms: f64,
) -> SynaraAgentApprovalHistoryContent {
    let mut items = content.items;
    items.push(item);
    SynaraAgentApprovalHistoryContent {
        version: AGENT_APPROVAL_HISTORY_ACCOUNT_DATA_VERSION,
        items: prune_agent_approval_history_items(items, now_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(event_id: &str, decided_at: f64) -> SynaraAgentApprovalHistoryItem {
        SynaraAgentApprovalHistoryItem {
            room_id: "!room:example.org".to_owned(),
            event_id: event_id.to_owned(),
            sender: "@hermes:example.org".to_owned(),
            decision: SynaraAgentApprovalHistoryDecision::ApproveOnce,
            decided_at,
            origin_server_ts: decided_at - 1_000.0,
            expires_at: decided_at + 300_000.0,
            summary: "rm file".to_owned(),
        }
    }

    #[test]
    fn maps_contract_action_ids() {
        assert_eq!(
            SynaraAgentApprovalHistoryDecision::from_action_id(AGENT_APPROVAL_ACTION_APPROVE_ONCE),
            Some(SynaraAgentApprovalHistoryDecision::ApproveOnce)
        );
        assert_eq!(
            SynaraAgentApprovalHistoryDecision::from_action_id(
                AGENT_APPROVAL_ACTION_APPROVE_ALWAYS
            ),
            Some(SynaraAgentApprovalHistoryDecision::ApproveAlways)
        );
        assert_eq!(
            SynaraAgentApprovalHistoryDecision::from_action_id(AGENT_APPROVAL_ACTION_DENY),
            Some(SynaraAgentApprovalHistoryDecision::Deny)
        );
        assert_eq!(
            SynaraAgentApprovalHistoryDecision::from_action_id("agent-approval.review"),
            None
        );
    }

    #[test]
    fn checked_normalize_rejects_unknown_versions() {
        assert_eq!(
            normalize_agent_approval_history_content_checked(Some(&json!({ "items": [] })), 1.0),
            Err("agent-approval-history-unsupported-version")
        );
        assert_eq!(
            normalize_agent_approval_history_content_checked(
                Some(&json!({ "version": 2, "items": [] })),
                1.0
            ),
            Err("agent-approval-history-unsupported-version")
        );
        assert_eq!(
            normalize_agent_approval_history_content_checked(
                Some(&json!({ "version": 1, "items": {} })),
                1.0
            ),
            Err("agent-approval-history-invalid-items")
        );
    }

    #[test]
    fn missing_content_normalizes_to_empty_v1() {
        assert_eq!(
            normalize_agent_approval_history_content_checked(None, 1.0),
            Ok(SynaraAgentApprovalHistoryContent::default())
        );
    }

    #[test]
    fn drops_malformed_items_and_caps_summary() {
        let now = 1_700_000_000_000.0;
        let content = normalize_agent_approval_history_content(
            Some(&json!({
                "version": 1,
                "items": [
                    {
                        "roomId": "!room:example.org",
                        "eventId": "$ok",
                        "sender": "@hermes:example.org",
                        "decision": "deny",
                        "decidedAt": now,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": format!("{} extra", "x".repeat(MAX_AGENT_APPROVAL_HISTORY_SUMMARY_CHARS))
                    },
                    {
                        "roomId": "not-a-room",
                        "eventId": "$ok",
                        "sender": "@hermes:example.org",
                        "decision": "deny",
                        "decidedAt": now,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": "bad"
                    },
                    {
                        "roomId": "!room:example.org",
                        "eventId": "$ok2",
                        "sender": "@hermes:example.org",
                        "decision": "maybe",
                        "decidedAt": now,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": "bad"
                    }
                ]
            })),
            now,
        );
        assert_eq!(content.items.len(), 1);
        assert_eq!(content.items[0].event_id, "$ok");
        assert_eq!(
            content.items[0].summary.chars().count(),
            MAX_AGENT_APPROVAL_HISTORY_SUMMARY_CHARS
        );
        assert_eq!(
            content.items[0].decision,
            SynaraAgentApprovalHistoryDecision::Deny
        );
    }

    #[test]
    fn dedupes_by_room_and_event_keeping_newest_decision() {
        let now = 1_700_000_000_000.0;
        let first = item("$same", now - 5_000.0);
        let newer = SynaraAgentApprovalHistoryItem {
            decision: SynaraAgentApprovalHistoryDecision::Deny,
            decided_at: now,
            summary: "later".to_owned(),
            ..item("$same", now)
        };
        let other = item("$other", now - 1_000.0);
        let content = append_agent_approval_history_item(
            SynaraAgentApprovalHistoryContent {
                version: 1,
                items: vec![first, other],
            },
            newer,
            now,
        );
        assert_eq!(content.items.len(), 2);
        assert_eq!(content.items[0].event_id, "$same");
        assert_eq!(
            content.items[0].decision,
            SynaraAgentApprovalHistoryDecision::Deny
        );
        assert_eq!(content.items[0].summary, "later");
        assert_eq!(content.items[1].event_id, "$other");
    }

    #[test]
    fn drops_items_older_than_retention_and_caps_count() {
        let now = 1_700_000_000_000.0;
        let stale = item("$old", now - AGENT_APPROVAL_HISTORY_RETENTION_MS - 1.0);
        let mut items: Vec<_> = (0..MAX_AGENT_APPROVAL_HISTORY_ITEMS + 5)
            .map(|index| item(&format!("$keep{index}"), now - index as f64))
            .collect();
        items.push(stale);
        let pruned = prune_agent_approval_history_items(items, now);
        assert_eq!(pruned.len(), MAX_AGENT_APPROVAL_HISTORY_ITEMS);
        assert!(pruned.iter().all(|item| item.event_id != "$old"));
        assert_eq!(pruned[0].event_id, "$keep0");
        assert!(pruned
            .windows(2)
            .all(|pair| pair[0].decided_at >= pair[1].decided_at));
    }

    #[test]
    fn serialize_uses_contract_field_names() {
        let encoded = serde_json::to_value(item("$event", 10_000.0)).expect("encode");
        assert_eq!(encoded["roomId"], "!room:example.org");
        assert_eq!(encoded["eventId"], "$event");
        assert_eq!(encoded["decision"], "approve_once");
        assert_eq!(encoded["decidedAt"], 10_000.0);
        assert_eq!(encoded["originServerTs"], 9_000.0);
        assert_eq!(encoded["expiresAt"], 310_000.0);
        assert!(encoded.get("command").is_none());
    }

    #[test]
    fn hostile_account_data_items_are_dropped_or_sanitized() {
        let now = 1_700_000_000_000.0;
        let content = normalize_agent_approval_history_content(
            Some(&json!({
                "version": 1,
                "items": [
                    {
                        "roomId": "!room:example.org",
                        "eventId": "$ok",
                        "sender": "@hermes:example.org",
                        "decision": "deny",
                        "decidedAt": now,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": "rm file"
                    },
                    {
                        "roomId": "!ApprovedAlways",
                        "eventId": "$spoof-room",
                        "sender": "@hermes:example.org",
                        "decision": "approve_always",
                        "decidedAt": now - 4.0,
                        "originServerTs": now - 5.0,
                        "expiresAt": now + 1.0,
                        "summary": "spoofed room"
                    },
                    {
                        "roomId": "!room:example.org",
                        "eventId": "$spoof-sender",
                        "sender": "You approved this",
                        "decision": "approve_always",
                        "decidedAt": now - 3.0,
                        "originServerTs": now - 4.0,
                        "expiresAt": now + 1.0,
                        "summary": "spoofed sender"
                    },
                    {
                        "roomId": "!room:example.org",
                        "eventId": "$bidi",
                        "sender": "@hermes:example.org",
                        "decision": "deny",
                        "decidedAt": now - 1.0,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": "rm \u{202E}elif"
                    },
                    {
                        "roomId": "!room:example.org",
                        "eventId": "$negative",
                        "sender": "@hermes:example.org",
                        "decision": "deny",
                        "decidedAt": -1.0,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": "negative"
                    },
                    {
                        "roomId": "!room:example.org",
                        "eventId": 12,
                        "sender": "@hermes:example.org",
                        "decision": "deny",
                        "decidedAt": now,
                        "originServerTs": now - 1.0,
                        "expiresAt": now + 1.0,
                        "summary": "non-string"
                    }
                ]
            })),
            now,
        );
        assert_eq!(content.items.len(), 2);
        assert_eq!(content.items[0].event_id, "$ok");
        assert_eq!(content.items[0].summary, "rm file");
        assert_eq!(content.items[1].event_id, "$bidi");
        assert_eq!(content.items[1].summary, "rm elif");
        assert!(!content.items[1].summary.contains('\u{202E}'));
    }
}
