//! Stable search-related compile-only API-shape probes.
//!
//! Server-side room message search has no dedicated high-level stable API in
//! this feature set; that candidate is deferred to P0.3c (typed `Client::send`
//! residual path / experimental local search).
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::Client;
use matrix_sdk::room_directory_search::RoomDirectorySearch;
use matrix_sdk::ruma::api::client::user_directory::search_users;

/// P0.3b-client-search-users — `Client::search_users`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn search_users`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_search_users() {
    async fn _shape(
        client: &Client,
        search_term: &str,
        limit: u64,
    ) -> matrix_sdk::HttpResult<search_users::v3::Response> {
        client.search_users(search_term, limit).await
    }
    let _ = _shape;
}

/// P0.3b-room-directory-search-type — `RoomDirectorySearch` is a public type.
///
/// Source: `crates/matrix-sdk/src/room_directory_search.rs`
/// (`pub struct RoomDirectorySearch`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_directory_search_type() -> &'static str {
    std::any::type_name::<RoomDirectorySearch>()
}

/// P0.3b-room-directory-search-new — `RoomDirectorySearch::new(Client)`.
///
/// Source: `crates/matrix-sdk/src/room_directory_search.rs` (`pub fn new`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_directory_search_new() {
    let _ctor: fn(Client) -> RoomDirectorySearch = RoomDirectorySearch::new;
    let _ = _ctor;
}

/// Run every stable search probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_client_search_users();
    let _ = probe_room_directory_search_type();
    probe_room_directory_search_new();
}
