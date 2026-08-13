//! Live public-directory search, cancel, and request-authority.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use matrix_sdk::ruma::{
    api::client::directory::get_public_rooms_filtered,
    directory::{Filter, RoomNetwork, RoomTypeFilter},
    OwnedServerName,
};
use matrix_sdk::Client;

use super::{
    normalize_search_input, DirectoryRoomHit, DirectoryRoomHitDto, DirectoryRoomType,
    DirectorySearchInput, NativeRoomDirectoryPage, NativeRoomDirectorySearchResponse,
    NormalizedDirectorySearch, RoomDirectorySession, MAX_DIRECTORY_HITS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestAuthority {
    Current,
    Stale,
    Cancelled,
}

#[derive(Default)]
struct DirectoryAuthority {
    current: BTreeMap<u64, u64>,
    cancelled: BTreeSet<(u64, u64)>,
}

const MAX_TRACKED_GENERATIONS: usize = 32;

fn authority() -> &'static Mutex<DirectoryAuthority> {
    static AUTHORITY: OnceLock<Mutex<DirectoryAuthority>> = OnceLock::new();
    AUTHORITY.get_or_init(|| Mutex::new(DirectoryAuthority::default()))
}

pub fn register_request(session_generation: u64, request_id: u64) -> RequestAuthority {
    let mut authority = authority()
        .lock()
        .expect("directory authority is not poisoned");
    if authority
        .cancelled
        .contains(&(session_generation, request_id))
    {
        return RequestAuthority::Cancelled;
    }
    match authority.current.get(&session_generation).copied() {
        Some(current) if request_id <= current => RequestAuthority::Stale,
        _ => {
            authority.current.insert(session_generation, request_id);
            while authority.current.len() > MAX_TRACKED_GENERATIONS {
                let Some(oldest_generation) = authority.current.keys().next().copied() else {
                    break;
                };
                authority.current.remove(&oldest_generation);
            }
            RequestAuthority::Current
        }
    }
}

pub fn cancel_request(session_generation: u64, request_id: u64) {
    let mut authority = authority()
        .lock()
        .expect("directory authority is not poisoned");
    {
        let current = authority
            .current
            .entry(session_generation)
            .or_insert(request_id);
        if request_id >= *current {
            *current = request_id;
        }
    }
    while authority.current.len() > MAX_TRACKED_GENERATIONS {
        let Some(oldest_generation) = authority.current.keys().next().copied() else {
            break;
        };
        authority.current.remove(&oldest_generation);
    }
    authority.cancelled.insert((session_generation, request_id));
    if authority.cancelled.len() > 256 {
        let keep = authority
            .cancelled
            .iter()
            .rev()
            .take(128)
            .copied()
            .collect::<BTreeSet<_>>();
        authority.cancelled = keep;
    }
}

pub fn request_authority(session_generation: u64, request_id: u64) -> RequestAuthority {
    let authority = authority()
        .lock()
        .expect("directory authority is not poisoned");
    if authority
        .cancelled
        .contains(&(session_generation, request_id))
    {
        return RequestAuthority::Cancelled;
    }
    match authority.current.get(&session_generation).copied() {
        Some(current) if current == request_id => RequestAuthority::Current,
        _ => RequestAuthority::Stale,
    }
}

pub fn cancelled_response(
    session_generation: u64,
    request_id: u64,
) -> NativeRoomDirectorySearchResponse {
    NativeRoomDirectorySearchResponse {
        session_generation,
        request_id,
        status: "cancelled",
        page: None,
    }
}

pub fn stale_response(
    session_generation: u64,
    request_id: u64,
) -> NativeRoomDirectorySearchResponse {
    NativeRoomDirectorySearchResponse {
        session_generation,
        request_id,
        status: "stale",
        page: None,
    }
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
        Some(super::DirectoryRoomTypeFilter::Room) => vec![RoomTypeFilter::Default],
        Some(super::DirectoryRoomTypeFilter::Space) => vec![RoomTypeFilter::Space],
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
    if response.chunk.len() > MAX_DIRECTORY_HITS {
        return Err("v-rooms.directory-hit-cap");
    }
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

pub fn project_hit(
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

pub async fn search_directory(
    client: &Client,
    session_generation: u64,
    request_id: u64,
    input: DirectorySearchInput,
) -> Result<NativeRoomDirectorySearchResponse, &'static str> {
    let normalized = normalize_search_input(input)?;
    match register_request(session_generation, request_id) {
        RequestAuthority::Current => {}
        RequestAuthority::Stale => return Ok(stale_response(session_generation, request_id)),
        RequestAuthority::Cancelled => {
            return Ok(cancelled_response(session_generation, request_id))
        }
    }
    let request = build_public_rooms_request(&normalized)?;
    let response = client
        .public_rooms_filtered(request)
        .await
        .map_err(|_| "v-rooms.directory-sdk-failed")?;
    match request_authority(session_generation, request_id) {
        RequestAuthority::Stale => return Ok(stale_response(session_generation, request_id)),
        RequestAuthority::Cancelled => {
            return Ok(cancelled_response(session_generation, request_id))
        }
        RequestAuthority::Current => {}
    }
    let page = project_response(session_generation, request_id, &normalized, response)?;
    Ok(NativeRoomDirectorySearchResponse {
        session_generation,
        request_id,
        status: "ready",
        page: Some(page),
    })
}

pub fn cancel_directory(
    session_generation: u64,
    request_id: u64,
) -> NativeRoomDirectorySearchResponse {
    cancel_request(session_generation, request_id);
    cancelled_response(session_generation, request_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::room_directory::{DirectoryRoomTypeFilter, DirectorySearchInput};

    #[test]
    fn request_authority_requires_strictly_increasing_ids() {
        let generation = 9_000_000_001;
        assert_eq!(register_request(generation, 1), RequestAuthority::Current);
        assert_eq!(register_request(generation, 1), RequestAuthority::Stale);
        assert_eq!(register_request(generation, 2), RequestAuthority::Current);
        assert_eq!(request_authority(generation, 1), RequestAuthority::Stale);
        assert_eq!(request_authority(generation, 2), RequestAuthority::Current);
    }

    #[test]
    fn default_room_type_is_projected_from_ruma_default() {
        let hit: matrix_sdk::ruma::directory::PublicRoomsChunk =
            matrix_sdk::ruma::directory::PublicRoomsChunkInit {
                num_joined_members: matrix_sdk::ruma::uint!(0),
                room_id: matrix_sdk::ruma::room_id!("!room:example.org").to_owned(),
                world_readable: true,
                guest_can_join: true,
            }
            .into();
        assert_eq!(project_hit(hit).unwrap().room_type, DirectoryRoomType::Room);
    }

    #[test]
    fn space_room_type_is_projected_and_custom_types_fail_closed() {
        let mut space: matrix_sdk::ruma::directory::PublicRoomsChunk =
            matrix_sdk::ruma::directory::PublicRoomsChunkInit {
                num_joined_members: matrix_sdk::ruma::uint!(0),
                room_id: matrix_sdk::ruma::room_id!("!space:example.org").to_owned(),
                world_readable: true,
                guest_can_join: true,
            }
            .into();
        space.room_type = Some(matrix_sdk::ruma::room::RoomType::Space);
        assert_eq!(
            project_hit(space).unwrap().room_type,
            DirectoryRoomType::Space
        );

        let mut custom: matrix_sdk::ruma::directory::PublicRoomsChunk =
            matrix_sdk::ruma::directory::PublicRoomsChunkInit {
                num_joined_members: matrix_sdk::ruma::uint!(0),
                room_id: matrix_sdk::ruma::room_id!("!custom:example.org").to_owned(),
                world_readable: true,
                guest_can_join: true,
            }
            .into();
        custom.room_type = Some(matrix_sdk::ruma::room::RoomType::from("org.example.custom"));
        assert_eq!(
            project_hit(custom).unwrap_err(),
            "v-rooms.directory-unsupported-room-type"
        );
    }

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
