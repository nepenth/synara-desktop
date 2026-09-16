//! Credential-free room join-rule presentation DTO.
//!
//! Live subscribe lives in [`super::live`]; shells map updates onto their emit sink.

use serde::{Deserialize, Serialize};

/// Tauri event: join rule may have changed; UI re-reads via existing snapshot IPC.
pub const ROOM_JOIN_RULE_UPDATED_EVENT: &str = "matrix-room-join-rule-updated";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NativeRoomJoinRuleUpdate {
    Ready {
        #[serde(rename = "roomId")]
        room_id: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
        #[serde(rename = "joinRule")]
        join_rule: &'static str,
    },
    Unavailable {
        #[serde(rename = "roomId")]
        room_id: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomDirectoryVisibilityResult {
    pub status: &'static str,
    pub room_id: String,
    pub session_generation: u64,
    pub visibility: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomDirectoryVisibilityWriteResult {
    pub status: &'static str,
    pub room_id: String,
    pub session_generation: u64,
    pub requested_visibility: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomJoinRuleSnapshot {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub join_rule: String,
}

/// Read-only MSC1763 retention projection. Unknown is not "kept forever."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomRetentionSnapshot {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub advertised: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lifetime_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_lifetime_ms: Option<u64>,
    pub summary: String,
    pub distinction: String,
    pub media_cache_summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn update_wire_shape_is_exact_and_camel_case() {
        let value = serde_json::to_value(NativeRoomJoinRuleUpdate::Ready {
            room_id: "!room:example.org".into(),
            session_generation: 7,
            join_rule: "knock_restricted",
        })
        .unwrap();
        assert_eq!(
            value,
            json!({
                "status": "ready",
                "roomId": "!room:example.org",
                "sessionGeneration": 7,
                "joinRule": "knock_restricted",
            })
        );

        let unavailable = serde_json::to_value(NativeRoomJoinRuleUpdate::Unavailable {
            room_id: "!room:example.org".into(),
            session_generation: 7,
        })
        .unwrap();
        assert_eq!(
            unavailable,
            json!({
                "status": "unavailable",
                "roomId": "!room:example.org",
                "sessionGeneration": 7,
            })
        );
    }

    #[test]
    fn retention_snapshot_wire_shape_is_camel_case_without_raw_policy() {
        let value = serde_json::to_value(MatrixRoomRetentionSnapshot {
            status: "ok".into(),
            room_id: "!room:example.org".into(),
            session_generation: 7,
            advertised: true,
            max_lifetime_ms: Some(86_400_000),
            min_lifetime_ms: None,
            summary: "This room's server may delete messages older than 1 day.".into(),
            distinction: "This is separate from history visibility.".into(),
            media_cache_summary: "Media cache on this device expires after 30 days.".into(),
        })
        .unwrap();
        assert_eq!(value["roomId"], "!room:example.org");
        assert_eq!(value["sessionGeneration"], 7);
        assert_eq!(value["maxLifetimeMs"], 86_400_000);
        assert!(value.get("policies").is_none());
        assert!(value.get("token").is_none());
    }
}
