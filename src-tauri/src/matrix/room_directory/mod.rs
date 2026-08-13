//! P6.10 — Public room directory search and projection owner.
#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::room_directory::*;

mod live;
pub use live::{
    build_public_rooms_request, fetch_protocols, normalize_search_input, project_hit,
    project_protocols, project_response, DirectoryProtocolInstance, DirectoryRoomHitDto,
    DirectoryRoomTypeFilter, DirectorySearchInput, NativeRoomDirectoryPage,
    NativeRoomDirectoryProtocols, NativeRoomDirectorySearchResponse, NormalizedDirectorySearch,
    MAX_PROTOCOL_INSTANCES,
};
