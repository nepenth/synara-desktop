//! Sync and room-list compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use std::sync::Arc;

use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::RoomId;
use matrix_sdk::sync::SyncResponse;
use matrix_sdk::{Client, Room, RoomState};
use matrix_sdk_ui::RoomListService;
use matrix_sdk_ui::room_list_service::RoomList;
use matrix_sdk_ui::sync_service::SyncService;

/// P0.3b-sync-service-start — `SyncService::start`.
///
/// Source: `crates/matrix-sdk-ui/src/sync_service.rs` (`pub async fn start`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_sync_service_start() {
    async fn _shape(svc: &SyncService) {
        svc.start().await;
    }
    let _ = _shape;
}

/// P0.3b-sync-service-room-list-service — `SyncService::room_list_service`.
///
/// Source: `crates/matrix-sdk-ui/src/sync_service.rs`
/// (`pub fn room_list_service(&self) -> Arc<RoomListService>`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_sync_service_room_list_service() {
    fn _shape(svc: &SyncService) -> Arc<RoomListService> {
        svc.room_list_service()
    }
    let _ = _shape;
}

/// P0.3b-client-sync-once — `Client::sync_once`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn sync_once`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_sync_once() {
    async fn _shape(client: &Client, settings: SyncSettings) -> matrix_sdk::Result<SyncResponse> {
        client.sync_once(settings).await
    }
    let _ = _shape;
}

/// P0.3b-room-list-service-all-rooms — `RoomListService::all_rooms`.
///
/// Source: `crates/matrix-sdk-ui/src/room_list_service/mod.rs`
/// (`pub async fn all_rooms`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_list_service_all_rooms() {
    async fn _shape(
        svc: &RoomListService,
    ) -> Result<RoomList, matrix_sdk_ui::room_list_service::Error> {
        svc.all_rooms().await
    }
    let _ = _shape;
}

/// P0.3b-client-get-room — `Client::get_room`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn get_room`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_get_room() {
    fn _shape(client: &Client, room_id: &RoomId) -> Option<Room> {
        client.get_room(room_id)
    }
    let _ = _shape;
}

/// P0.3b-client-rooms — `Client::rooms`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn rooms`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_rooms() {
    fn _shape(client: &Client) -> Vec<Room> {
        client.rooms()
    }
    let _ = _shape;
}

/// P0.3b-client-joined-rooms — `Client::joined_rooms`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn joined_rooms`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_joined_rooms() {
    fn _shape(client: &Client) -> Vec<Room> {
        client.joined_rooms()
    }
    let _ = _shape;
}

/// P0.3b-room-state — `Room.state()` via `Deref` to `BaseRoom`.
///
/// Source: `crates/matrix-sdk-base/src/room/mod.rs` (`pub fn state`) reachable
/// on `matrix_sdk::Room` via `Deref`.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_state() {
    fn _shape(room: &Room) -> RoomState {
        room.state()
    }
    let _ = _shape;
}

/// Run every sync/room-list probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_sync_service_start();
    probe_sync_service_room_list_service();
    probe_client_sync_once();
    probe_room_list_service_all_rooms();
    probe_client_get_room();
    probe_client_rooms();
    probe_client_joined_rooms();
    probe_room_state();
}
