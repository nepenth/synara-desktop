//! Credential-free V-ROOMS.4 typing presentation DTO.
//!
//! Live Client `m.typing` ownership lives in [`super::live`].

use serde::{Deserialize, Serialize};

use crate::dto::TypingSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTypingSnapshot {
    pub session_generation: u64,
    pub rooms: Vec<TypingSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_snapshot_serializes_camel_case() {
        let snap = NativeTypingSnapshot {
            session_generation: 3,
            rooms: vec![TypingSnapshot {
                room_id: "!r:example.org".into(),
                user_ids: vec!["@alice:example.org".into()],
            }],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 3);
        assert_eq!(value["rooms"][0]["roomId"], "!r:example.org");
        assert_eq!(value["rooms"][0]["userIds"][0], "@alice:example.org");
    }
}
