//! Compile-only public API-shape probes for `matrix-sdk` / `matrix-sdk-ui` 0.18.0.
//!
//! Each probe below forces the compiler to resolve named public types and
//! function signatures. **These probes do not prove runtime or network
//! semantics.** They never connect to a homeserver, open a store, or handle
//! secrets.
//!
//! Upstream pin: tag `matrix-sdk-0.18.0`, commit
//! `1c44fb66214667c6d00acaf72ab592493653708b`.

#![forbid(unsafe_code)]
#![deny(unreachable_pub)]

use matrix_sdk::{Client, ClientBuilder, Room};
use matrix_sdk_ui::timeline::{RoomExt, TimelineBuilder};
use matrix_sdk_ui::{
    RoomListService, Timeline,
    sync_service::{SyncService, SyncServiceBuilder},
};

/// Probe catalog (stable IDs used by provenance docs).
pub const PROBE_IDS: &[&str] = &[
    "P0.3a-client-type",
    "P0.3a-client-builder-type",
    "P0.3a-client-builder-fn",
    "P0.3a-room-type",
    "P0.3a-room-room-id-fn",
    "P0.3a-sync-service-type",
    "P0.3a-sync-service-builder-fn",
    "P0.3a-room-list-service-type",
    "P0.3a-timeline-type",
    "P0.3a-timeline-builder-type",
    "P0.3a-room-ext-timeline-builder",
];

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

/// P0.3a-room-room-id-fn — `Room.room_id()` is reachable on the public `Room`
/// type via `Deref` to `matrix_sdk_base::Room` (`BaseRoom`).
///
/// Source (pinned commit):
/// - `crates/matrix-sdk/src/room/mod.rs` (`impl Deref for Room`)
/// - `crates/matrix-sdk-base/src/room/mod.rs` (`pub fn room_id(&self) -> &RoomId`)
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
/// UFCS (`Room::room_id`) is intentionally not used: Deref methods are only
/// available via method-call syntax.
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
/// (`pub struct SyncService`). Not re-exported at the UI crate root.
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
/// (`pub struct RoomListService`) and crate-root re-export in
/// `crates/matrix-sdk-ui/src/lib.rs`.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_list_service_type() -> &'static str {
    std::any::type_name::<RoomListService>()
}

/// P0.3a-timeline-type — `matrix_sdk_ui::Timeline`.
///
/// Source (pinned commit): `crates/matrix-sdk-ui/src/timeline/mod.rs`
/// (`pub struct Timeline`) and crate-root re-export in
/// `crates/matrix-sdk-ui/src/lib.rs`.
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

/// Run every probe function so `cargo test` exercises the monomorphized shapes.
///
/// Still compile-only: no network, no stores, no secrets.
pub fn run_all_probes() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_api_shape_probes_compile_and_run() {
        // Compile-only API-shape probe execution; does not prove runtime/network semantics.
        run_all_probes();
        assert_eq!(PROBE_IDS.len(), 11);
        assert!(probe_client_type().contains("Client"));
        assert!(probe_client_builder_type().contains("ClientBuilder"));
        assert!(probe_room_type().contains("Room"));
        assert!(probe_sync_service_type().contains("SyncService"));
        assert!(probe_room_list_service_type().contains("RoomListService"));
        assert!(probe_timeline_type().contains("Timeline"));
        assert!(probe_timeline_builder_type().contains("TimelineBuilder"));
    }
}
