//! Experimental local message-search index probes (`experimental-search`).
//!
//! Profile: `profile-experimental-search`.
//! Compile-only; does not open indexes, stores, or perform I/O.

use matrix_sdk::message_search::GlobalSearchBuilder;
use matrix_sdk::search_index::{SearchIndex, SearchIndexStoreKind};
use matrix_sdk::{Client, ClientBuilder, Room};

/// Probe IDs compiled under `profile-experimental-search`.
pub const PROBE_IDS: &[&str] = &[
    "P0.3c-search-index-type",
    "P0.3c-search-index-store-kind-type",
    "P0.3c-client-search-index",
    "P0.3c-client-builder-search-index-store",
    "P0.3c-room-search-local-index",
    "P0.3c-client-search-messages",
];

/// P0.3c-search-index-type
pub fn probe_search_index_type() -> &'static str {
    std::any::type_name::<SearchIndex>()
}

/// P0.3c-search-index-store-kind-type
pub fn probe_search_index_store_kind_type() -> &'static str {
    std::any::type_name::<SearchIndexStoreKind>()
}

/// P0.3c-client-search-index
pub fn probe_client_search_index() {
    fn _shape(client: &Client) -> &SearchIndex {
        client.search_index()
    }
    let _ = _shape;
}

/// P0.3c-client-builder-search-index-store
///
/// Configures local index persistence kind (InMemory / directory / encrypted).
/// Does not open a store in this probe.
pub fn probe_client_builder_search_index_store() {
    fn _shape(builder: ClientBuilder, kind: SearchIndexStoreKind) -> ClientBuilder {
        builder.search_index_store(kind)
    }
    let _ = _shape;
}

/// P0.3c-room-search-local-index — local RoomIndex search (not server /search).
pub fn probe_room_search_local_index() {
    async fn _shape(
        room: &Room,
        query: &str,
        max_number_of_results: usize,
        pagination_offset: Option<usize>,
    ) {
        let _ = room
            .search(query, max_number_of_results, pagination_offset)
            .await;
    }
    let _ = _shape;
}

/// P0.3c-client-search-messages — multi-room local index helper entry.
pub fn probe_client_search_messages() {
    fn _shape(client: &Client, query: String, max: usize) -> GlobalSearchBuilder {
        client.search_messages(query, max)
    }
    let _ = _shape;
}

/// Run every experimental-search probe (compile-only).
pub fn run_all() {
    let _ = probe_search_index_type();
    let _ = probe_search_index_store_kind_type();
    probe_client_search_index();
    probe_client_builder_search_index_store();
    probe_room_search_local_index();
    probe_client_search_messages();
}
