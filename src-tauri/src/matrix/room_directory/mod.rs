//! P6.10 — Public room directory search and projection owner.
//!
//! The live product command module is included from the managed auth product
//! boundary, while the typed request/projection helpers remain module-owned.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.10-room-directory.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod live;
mod session;

pub use error::RoomDirectoryError;
pub use live::{
    build_public_rooms_request, fetch_protocols, normalize_search_input, project_protocols,
    project_response, DirectoryProtocolInstance, DirectoryRoomHitDto, DirectoryRoomTypeFilter,
    DirectorySearchInput, NativeRoomDirectoryPage, NativeRoomDirectoryProtocols,
    NativeRoomDirectorySearchResponse, NormalizedDirectorySearch, MAX_PROTOCOL_INSTANCES,
};
pub use session::{
    DirectoryRoomHit, DirectoryRoomType, DirectorySearchState, RoomDirectorySession,
    MAX_ALIAS_CHARS, MAX_BATCH_CHARS, MAX_DIRECTORY_HITS, MAX_TEXT_CHARS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_ROOM_DIRECTORY_MARKER: &str = "matrix-room-directory-p6.10";

/// Touch room-directory paths so they remain linked in non-test builds.
pub fn matrix_room_directory_markers() -> &'static str {
    let s = RoomDirectorySession::new(0);
    debug_assert_eq!(s.state(), DirectorySearchState::Idle);
    debug_assert_eq!(MAX_DIRECTORY_HITS, 200);
    debug_assert_eq!(MATRIX_ROOM_DIRECTORY_MARKER, "matrix-room-directory-p6.10");
    MATRIX_ROOM_DIRECTORY_MARKER
}

#[cfg(test)]
mod tests;
