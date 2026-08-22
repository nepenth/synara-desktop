//! Bounded homeserver message-search IPC DTOs.
//!
//! Result rows carry ids and a body snippet only. No raw event JSON, tokens,
//! or unbounded dumps.

use serde::{Deserialize, Serialize};

/// One search hit. Body is a capped plain-text snippet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMessageSearchItem {
    pub rank: f64,
    pub event_id: String,
    pub sender: String,
    pub origin_server_ts: u64,
    pub body: String,
    pub room_id: String,
}

/// Consecutive hits that share a room id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMessageSearchGroup {
    pub room_id: String,
    pub items: Vec<MatrixMessageSearchItem>,
}

/// Homeserver room-event search page. Highlights and groups are capped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMessageSearchResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    pub highlights: Vec<String>,
    pub groups: Vec<MatrixMessageSearchGroup>,
}
