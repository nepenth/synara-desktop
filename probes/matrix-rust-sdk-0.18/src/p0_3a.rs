//! P0.3a compile-only public API-shape probes (stable IDs preserved).
//!
//! Each probe forces the compiler to resolve named public types and function
//! signatures. **These probes do not prove runtime or network semantics.**

use matrix_sdk::{Client, ClientBuilder, Room};
use matrix_sdk_ui::timeline::{RoomExt, TimelineBuilder};
use matrix_sdk_ui::{
    RoomListService, Timeline,
    sync_service::{SyncService, SyncServiceBuilder},
};

/// P0.3a-client-type — `matrix_sdk::Client` is a public type.
///
/// Source (pinned commit): `crates/matrix-sdk/src/client/mod.rs` (`pub struct Client`)
/// and crate-root re-export in `crates/matrix-sdk/src/lib.rs`.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_type() -> &'static str {
    std::any::type_name::<Client>()
}

/// P0.3a-client-builder-type — `matrix_sdk::ClientBuilder` is a public type.
///
/// Source (pinned commit): `crates/matrix-sdk/src/client/builder/mod.rs`
/// (`pub struct ClientBuilder`) and crate-root re-export.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_builder_type() -> &'static str {
    std::any::type_name::<ClientBuilder>()
}

/// P0.3a-client-builder-fn — `Client::builder() -> ClientBuilder`.
///
/// Source (pinned commit): `crates/matrix-sdk/src/client/mod.rs`
/// (`pub fn builder() -> ClientBuilder`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_builder_fn() {
    let _ctor: fn() -> ClientBuilder = Client::builder;
    let _ = _ctor;
}

/// P0.3a-room-type — `matrix_sdk::Room` is a public type.
///
/// Source (pinned commit): `crates/matrix-sdk/src/room/mod.rs` (`pub struct Room`)
/// and crate-root re-export in `crates/matrix-sdk/src/lib.rs`.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_type() -> &'static str {
    std::any::type_name::<Room>()
}

/// P0.3a-room-room-id-fn — `Room.room_id()` via `Deref` to `BaseRoom`.
///
/// Source (pinned commit):
/// - `crates/matrix-sdk/src/room/mod.rs` (`impl Deref for Room`)
/// - `crates/matrix-sdk-base/src/room/mod.rs` (`pub fn room_id(&self) -> &RoomId`)
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_room_id_fn() {
    fn _shape(room: &Room) {
        let _id = room.room_id();
        let _ = _id.as_str();
    }
    let _ = _shape;
}

/// P0.3a-sync-service-type — `matrix_sdk_ui::sync_service::SyncService`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/sync_service.rs`
/// (`pub struct SyncService`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_sync_service_type() -> &'static str {
    std::any::type_name::<SyncService>()
}

/// P0.3a-sync-service-builder-fn — `SyncService::builder(Client) -> SyncServiceBuilder`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/sync_service.rs`
/// (`pub fn builder(client: Client) -> SyncServiceBuilder`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_sync_service_builder_fn() {
    let _ctor: fn(Client) -> SyncServiceBuilder = SyncService::builder;
    let _ = _ctor;
}

/// P0.3a-room-list-service-type — `matrix_sdk_ui::RoomListService`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/room_list_service/mod.rs`
/// (`pub struct RoomListService`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_list_service_type() -> &'static str {
    std::any::type_name::<RoomListService>()
}

/// P0.3a-timeline-type — `matrix_sdk_ui::Timeline`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/timeline/mod.rs`
/// (`pub struct Timeline`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_type() -> &'static str {
    std::any::type_name::<Timeline>()
}

/// P0.3a-timeline-builder-type — `matrix_sdk_ui::timeline::TimelineBuilder`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/timeline/builder.rs`
/// (`pub struct TimelineBuilder`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_builder_type() -> &'static str {
    std::any::type_name::<TimelineBuilder>()
}

/// P0.3a-room-ext-timeline-builder — `RoomExt::timeline_builder(&Room) -> TimelineBuilder`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/timeline/traits.rs`
/// (`pub trait RoomExt`, `fn timeline_builder(&self) -> TimelineBuilder`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_ext_timeline_builder() {
    let _method: fn(&Room) -> TimelineBuilder = RoomExt::timeline_builder;
    let _ = _method;
}

/// Run every P0.3a probe (still compile-only: no network, stores, or secrets).
pub fn run_all() {
    let _ = probe_client_type();
    let _ = probe_client_builder_type();
    probe_client_builder_fn();
    let _ = probe_room_type();
    probe_room_room_id_fn();
    let _ = probe_sync_service_type();
    probe_sync_service_builder_fn();
    let _ = probe_room_list_service_type();
    let _ = probe_timeline_type();
    let _ = probe_timeline_builder_type();
    probe_room_ext_timeline_builder();
}
