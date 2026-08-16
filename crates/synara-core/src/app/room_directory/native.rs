//! Credential-free V-ROOMS directory presentation DTOs and input normalize.
//!
//! Live protocol listing lives in `live.rs`. Search request mapping stays desktop.

use serde::{Deserialize, Serialize};

use super::session::{DirectoryRoomHit, DirectoryRoomType, MAX_BATCH_CHARS, MAX_TEXT_CHARS};

pub const MAX_PROTOCOL_INSTANCES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryRoomTypeFilter {
    Room,
    Space,
}

#[derive(Debug, Clone, Default)]
pub struct DirectorySearchInput {
    pub server_name: Option<String>,
    pub term: Option<String>,
    pub room_type: Option<DirectoryRoomTypeFilter>,
    pub third_party_instance_id: Option<String>,
    pub limit: u64,
    pub since: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NormalizedDirectorySearch {
    pub server_name: Option<String>,
    pub term: Option<String>,
    pub room_type: Option<DirectoryRoomTypeFilter>,
    pub third_party_instance_id: Option<String>,
    pub limit: u64,
    pub since: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryProtocolInstance {
    pub protocol_id: String,
    pub instance_id: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomDirectoryProtocols {
    pub session_generation: u64,
    pub instances: Vec<DirectoryProtocolInstance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomDirectoryPage {
    pub session_generation: u64,
    pub request_id: u64,
    pub chunk: Vec<DirectoryRoomHitDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_batch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_batch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRoomHitDto {
    pub room_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub member_count: u32,
    pub world_readable: bool,
    pub guest_can_join: bool,
    pub room_type: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomDirectorySearchResponse {
    pub session_generation: u64,
    pub request_id: u64,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<NativeRoomDirectoryPage>,
}

pub fn normalize_search_input(
    input: DirectorySearchInput,
) -> Result<NormalizedDirectorySearch, &'static str> {
    if input.limit == 0 || input.limit > 100 {
        return Err("v-rooms.directory-invalid-limit");
    }
    let server_name = normalize_optional(
        input.server_name,
        MAX_TEXT_CHARS,
        "v-rooms.directory-invalid-server",
    )?;
    let term = normalize_optional(input.term, MAX_TEXT_CHARS, "v-rooms.directory-invalid-term")?;
    let third_party_instance_id = normalize_optional(
        input.third_party_instance_id,
        MAX_TEXT_CHARS,
        "v-rooms.directory-invalid-instance",
    )?;
    let since = normalize_optional(
        input.since,
        MAX_BATCH_CHARS,
        "v-rooms.directory-invalid-since",
    )?;
    Ok(NormalizedDirectorySearch {
        server_name,
        term,
        room_type: input.room_type,
        third_party_instance_id,
        limit: input.limit,
        since,
    })
}

fn normalize_optional(
    value: Option<String>,
    max_chars: usize,
    diagnostic_id: &'static str,
) -> Result<Option<String>, &'static str> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_chars {
        return Err(diagnostic_id);
    }
    if value.contains("access_token") || value.contains("refresh_token") {
        return Err(diagnostic_id);
    }
    Ok(Some(value.to_owned()))
}

impl From<&DirectoryRoomHit> for DirectoryRoomHitDto {
    fn from(hit: &DirectoryRoomHit) -> Self {
        Self {
            room_id: hit.room_id.clone(),
            name: hit.name.clone(),
            topic: hit.topic.clone(),
            canonical_alias: hit.canonical_alias.clone(),
            avatar_url: hit.avatar_url.clone(),
            member_count: hit.num_joined_members,
            world_readable: hit.world_readable,
            guest_can_join: hit.guest_can_join,
            room_type: match hit.room_type {
                DirectoryRoomType::Room => "room",
                DirectoryRoomType::Space => "space",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_inputs_are_trimmed_and_empty_values_are_omitted() {
        let normalized = normalize_search_input(DirectorySearchInput {
            server_name: Some("  example.org  ".into()),
            term: Some("  ".into()),
            third_party_instance_id: Some("  ".into()),
            since: Some("  ".into()),
            limit: 24,
            ..DirectorySearchInput::default()
        })
        .unwrap();
        assert_eq!(normalized.server_name.as_deref(), Some("example.org"));
        assert_eq!(normalized.term, None);
        assert_eq!(normalized.third_party_instance_id, None);
        assert_eq!(normalized.since, None);
    }
}
