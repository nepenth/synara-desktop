//! Message search DTOs — result rows with optional snippets only.

use serde::{Deserialize, Serialize};

use super::ids::{EventId, RoomId, UserId};

/// Single search hit (product projection).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    pub event_id: EventId,
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_server_ts: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<UserId>,
    /// Privacy-safe plain-text snippet (no ciphertext dump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// Search response / page projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,
    pub results: Vec<SearchResultItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_batch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
}
