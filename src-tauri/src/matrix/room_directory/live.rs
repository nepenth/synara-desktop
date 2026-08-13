//! Typed Matrix SDK request and bounded product projection for the public
//! room directory. SDK/Ruma values stop in this module.

use matrix_sdk::ruma::{
    api::client::directory::get_public_rooms_filtered,
    directory::{Filter, RoomNetwork, RoomTypeFilter},
    OwnedServerName,
};

use super::{DirectoryRoomHit, DirectoryRoomType, RoomDirectorySession, MAX_DIRECTORY_HITS};

pub use synara_core::app::room_directory::{
    fetch_protocols, normalize_search_input, project_protocols, DirectoryProtocolInstance,
    DirectoryRoomHitDto, DirectoryRoomTypeFilter, DirectorySearchInput, NativeRoomDirectoryPage,
    NativeRoomDirectoryProtocols, NativeRoomDirectorySearchResponse, NormalizedDirectorySearch,
    MAX_PROTOCOL_INSTANCES,
};

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

fn project_hit(
    hit: matrix_sdk::ruma::directory::PublicRoomsChunk,
) -> Result<DirectoryRoomHit, &'static str> {
    let room_type = match hit.room_type.as_ref().map(|room_type| room_type.as_str()) {
        // Ruma represents the Matrix default room type by an absent
        // `room_type`. This is the typed default discriminator, not an
        // inference from arbitrary response data.
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

#[cfg(test)]
mod tests {
    use super::*;

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
