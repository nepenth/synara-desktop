//! Desktop re-export of Core directory live Client I/O.

pub use synara_core::app::room_directory::{
    build_public_rooms_request, fetch_protocols, normalize_search_input, project_hit,
    project_protocols, project_response, DirectoryProtocolInstance, DirectoryRoomHitDto,
    DirectoryRoomTypeFilter, DirectorySearchInput, NativeRoomDirectoryPage,
    NativeRoomDirectoryProtocols, NativeRoomDirectorySearchResponse, NormalizedDirectorySearch,
    MAX_PROTOCOL_INSTANCES,
};
