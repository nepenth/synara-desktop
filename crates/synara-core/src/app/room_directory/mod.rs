//! P6.10 — Public room directory search and projection (harness).
//!
//! Pure projection of directory hits plus live protocol listing and
//! search/cancel with the request-authority registry.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.10-room-directory.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod live;
mod native;
mod search;
mod session;

pub use error::RoomDirectoryError;
pub use live::{fetch_protocols, project_protocols};
pub use native::{
    canonicalize_directory_server_name, normalize_search_input, DirectoryProtocolInstance,
    DirectoryRoomHitDto, DirectoryRoomTypeFilter, DirectorySearchInput, NativeRoomDirectoryPage,
    NativeRoomDirectoryProtocols, NativeRoomDirectorySearchResponse, NormalizedDirectorySearch,
    MAX_PROTOCOL_INSTANCES,
};
pub use search::{
    build_public_rooms_request, cancel_directory, cancel_request, cancelled_response,
    directory_api_error_diagnostic, project_hit, project_response, register_request,
    request_authority, search_directory, stale_response, RequestAuthority,
};
pub use session::{
    directory_hit_is_presentable, DirectoryRoomHit, DirectoryRoomType, DirectorySearchState,
    RoomDirectorySession, MAX_ALIAS_CHARS, MAX_BATCH_CHARS, MAX_DIRECTORY_HITS, MAX_TEXT_CHARS,
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

#[cfg(test)]
mod fail_closed_session {
    #[test]
    fn project_response_rejects_invalid_hits_before_session_begin() {
        let source = include_str!("search.rs");
        let project = source
            .split("pub fn project_response(")
            .nth(1)
            .and_then(|rest| rest.split("pub fn project_hit(").next())
            .expect("project_response");
        let hits_err = project
            .find("collect::<Result<Vec<_>, &'static str>>()?")
            .expect("hit projection must fail closed");
        let begin = project.find(".begin(").expect("throwaway session begin");
        assert!(
            hits_err < begin,
            "invalid hits must not initialize directory session state"
        );
        assert!(project.contains("RoomDirectorySession::new(session_generation)"));
    }
}
