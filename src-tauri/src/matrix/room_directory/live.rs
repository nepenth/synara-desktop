//! Typed Matrix SDK request and bounded product projection for the public
//! room directory. SDK/Ruma values stop in this module.

use matrix_sdk::ruma::{
    api::client::{directory::get_public_rooms_filtered, thirdparty::get_protocols},
    directory::{Filter, RoomNetwork, RoomTypeFilter},
    OwnedServerName,
};
use matrix_sdk::Client;
use serde::Serialize;

use super::{
    DirectoryRoomHit, DirectoryRoomType, RoomDirectorySession, MAX_BATCH_CHARS, MAX_TEXT_CHARS,
};

pub const MAX_PROTOCOL_INSTANCES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryProtocolInstance {
    pub protocol_id: String,
    pub instance_id: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(diagnostic_id);
    }
    if value.contains("access_token") || value.contains("refresh_token") {
        return Err(diagnostic_id);
    }
    Ok(Some(value))
}

pub fn build_public_rooms_request(
    input: &NormalizedDirectorySearch,
) -> Result<get_public_rooms_filtered::v3::Request, &'static str> {
    let mut request = get_public_rooms_filtered::v3::Request::new();
    request.server = input
        .server_name
        .as_deref()
        .map(|server| OwnedServerName::try_from(server.to_owned()))
        .transpose()
        .map_err(|_| "v-rooms.directory-invalid-server")?;
    request.limit = Some(
        input
            .limit
            .try_into()
            .map_err(|_| "v-rooms.directory-invalid-limit")?,
    );
    request.since = input.since.clone();
    let mut filter = Filter::new();
    filter.generic_search_term = input.term.clone();
    filter.room_types = match input.room_type {
        None => Vec::new(),
        Some(DirectoryRoomTypeFilter::Room) => vec![RoomTypeFilter::Default],
        Some(DirectoryRoomTypeFilter::Space) => vec![RoomTypeFilter::Space],
    };
    request.filter = filter;
    request.room_network = input
        .third_party_instance_id
        .clone()
        .map(RoomNetwork::ThirdParty)
        .unwrap_or_default();
    Ok(request)
}

pub fn project_response(
    session_generation: u64,
    request_id: u64,
    input: &NormalizedDirectorySearch,
    response: get_public_rooms_filtered::v3::Response,
) -> Result<NativeRoomDirectoryPage, &'static str> {
    let hits = response
        .chunk
        .into_iter()
        .map(project_hit)
        .collect::<Result<Vec<_>, _>>()?;
    let mut session = RoomDirectorySession::new(session_generation);
    let internal_request_id = session
        .begin(
            input.term.clone().unwrap_or_default(),
            input.server_name.clone(),
        )
        .map_err(|error| error.diagnostic_id())?;
    session
        .apply_page_with_batches(
            internal_request_id,
            hits,
            response.prev_batch,
            response.next_batch,
            true,
        )
        .map_err(|error| error.diagnostic_id())?;
    Ok(NativeRoomDirectoryPage {
        session_generation,
        request_id,
        chunk: session
            .hits()
            .iter()
            .map(DirectoryRoomHitDto::from)
            .collect(),
        prev_batch: session.prev_batch().map(ToOwned::to_owned),
        next_batch: session.next_batch().map(ToOwned::to_owned),
    })
}

fn project_hit(
    hit: matrix_sdk::ruma::directory::PublicRoomsChunk,
) -> Result<DirectoryRoomHit, &'static str> {
    let room_type = match hit.room_type.as_ref().map(|room_type| room_type.as_str()) {
        None => DirectoryRoomType::Room,
        Some("m.space") => DirectoryRoomType::Space,
        Some(_) => return Err("v-rooms.directory-unsupported-room-type"),
    };
    let member_count: u64 = hit.num_joined_members.into();
    Ok(DirectoryRoomHit {
        room_id: hit.room_id.to_string(),
        name: hit.name,
        topic: hit.topic,
        canonical_alias: hit.canonical_alias.map(|alias| alias.to_string()),
        avatar_url: hit.avatar_url.map(|avatar| avatar.to_string()),
        num_joined_members: member_count.min(u32::MAX.into()) as u32,
        world_readable: hit.world_readable,
        guest_can_join: hit.guest_can_join,
        room_type,
    })
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

pub async fn fetch_protocols(
    client: &Client,
    session_generation: u64,
) -> Result<NativeRoomDirectoryProtocols, &'static str> {
    let response = client
        .send(get_protocols::v3::Request::new())
        .await
        .map_err(|_| "v-rooms.directory-protocols-sdk-failed")?;
    Ok(NativeRoomDirectoryProtocols {
        session_generation,
        instances: project_protocols(response.protocols)?,
    })
}

pub fn project_protocols(
    protocols: std::collections::BTreeMap<String, matrix_sdk::ruma::thirdparty::Protocol>,
) -> Result<Vec<DirectoryProtocolInstance>, &'static str> {
    let mut instances = Vec::new();
    for (protocol_id, protocol) in protocols {
        if protocol_id.chars().count() > MAX_TEXT_CHARS {
            return Err("v-rooms.directory-protocol-id-cap");
        }
        for instance in protocol.instances {
            let Some(instance_id) = instance.instance_id else {
                continue;
            };
            if protocol_id.is_empty()
                || instance_id.trim().is_empty()
                || instance_id.chars().count() > MAX_TEXT_CHARS
                || instance.desc.chars().count() > MAX_TEXT_CHARS
            {
                return Err("v-rooms.directory-protocol-instance-invalid");
            }
            instances.push(DirectoryProtocolInstance {
                protocol_id: protocol_id.clone(),
                instance_id,
                description: instance.desc,
            });
            if instances.len() > MAX_PROTOCOL_INSTANCES {
                return Err("v-rooms.directory-protocol-instance-cap");
            }
        }
    }
    Ok(instances)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_mapping_covers_filters_and_bounds() {
        let normalized = normalize_search_input(DirectorySearchInput {
            server_name: Some("example.org".into()),
            term: Some("rust".into()),
            room_type: Some(DirectoryRoomTypeFilter::Space),
            third_party_instance_id: Some("irc-example".into()),
            limit: 96,
            since: Some("next-1".into()),
        })
        .unwrap();
        let request = build_public_rooms_request(&normalized).unwrap();
        assert_eq!(request.server.unwrap().to_string(), "example.org");
        assert_eq!(request.filter.generic_search_term.as_deref(), Some("rust"));
        assert_eq!(request.filter.room_types[0].as_str(), Some("m.space"));
        assert_eq!(request.since.as_deref(), Some("next-1"));
        assert!(
            matches!(request.room_network, RoomNetwork::ThirdParty(ref id) if id == "irc-example")
        );
    }

    #[test]
    fn default_room_filter_maps_to_ruma_default() {
        let normalized = normalize_search_input(DirectorySearchInput {
            room_type: Some(DirectoryRoomTypeFilter::Room),
            limit: 1,
            ..DirectorySearchInput::default()
        })
        .unwrap();
        let request = build_public_rooms_request(&normalized).unwrap();
        assert_eq!(request.filter.room_types[0].as_str(), None);
    }
}
