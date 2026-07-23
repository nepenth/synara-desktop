//! Spaces / hierarchy compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::{Client, Room};
use matrix_sdk_ui::spaces::SpaceService;
use matrix_sdk_ui::spaces::room_list::SpaceRoomList;

/// P0.3b-client-joined-space-rooms — `Client::joined_space_rooms`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn joined_space_rooms`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_joined_space_rooms() {
    fn _shape(client: &Client) -> Vec<Room> {
        client.joined_space_rooms()
    }
    let _ = _shape;
}

/// P0.3b-space-service-type — `matrix_sdk_ui::spaces::SpaceService`.
///
/// Source: `crates/matrix-sdk-ui/src/spaces/mod.rs` (`pub struct SpaceService`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_space_service_type() -> &'static str {
    std::any::type_name::<SpaceService>()
}

/// P0.3b-space-service-new — `SpaceService::new(Client)`.
///
/// Source: `crates/matrix-sdk-ui/src/spaces/mod.rs` (`pub async fn new`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_space_service_new() {
    async fn _shape(client: Client) -> SpaceService {
        SpaceService::new(client).await
    }
    let _ = _shape;
}

/// P0.3b-space-room-list-type — `matrix_sdk_ui::spaces::room_list::SpaceRoomList`.
///
/// Source: `crates/matrix-sdk-ui/src/spaces/room_list.rs` (`pub struct SpaceRoomList`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_space_room_list_type() -> &'static str {
    std::any::type_name::<SpaceRoomList>()
}

/// P0.3b-space-service-space-room-list — `SpaceService::space_room_list`.
///
/// Source: `crates/matrix-sdk-ui/src/spaces/mod.rs` (`pub async fn space_room_list`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_space_service_space_room_list() {
    async fn _shape(svc: &SpaceService, space_id: OwnedRoomId) -> SpaceRoomList {
        svc.space_room_list(space_id).await
    }
    let _ = _shape;
}

/// Run every spaces probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_client_joined_space_rooms();
    let _ = probe_space_service_type();
    probe_space_service_new();
    let _ = probe_space_room_list_type();
    probe_space_service_space_room_list();
}
