//! Credential-free room join-rule presentation DTO.
//!
//! Live Client subscribe and SDK JoinRule mapping stay in the desktop shell.

use serde::Serialize;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRoomJoinRuleSnapshot {
    pub status: &'static str,
    pub room_id: String,
    pub session_generation: u64,
    pub join_rule: &'static str,
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
}
