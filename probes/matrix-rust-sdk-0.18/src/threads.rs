//! Threads / relations compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::Room;
use matrix_sdk::room::{Relations, RelationsOptions};
use matrix_sdk::ruma::OwnedEventId;
use matrix_sdk_ui::Timeline;
use matrix_sdk_ui::timeline::{RoomExt, ThreadListService};

/// P0.3b-room-relations — `Room::relations`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn relations`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_relations() {
    async fn _shape(
        room: &Room,
        event_id: OwnedEventId,
        opts: RelationsOptions,
    ) -> matrix_sdk::Result<Relations> {
        room.relations(event_id, opts).await
    }
    let _ = _shape;
}

/// P0.3b-thread-list-service-type — `ThreadListService` is a public type.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/thread_list_service.rs`
/// (`pub struct ThreadListService`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_thread_list_service_type() -> &'static str {
    std::any::type_name::<ThreadListService>()
}

/// P0.3b-room-ext-thread-list-service — `RoomExt::thread_list_service`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/traits.rs`
/// (`fn thread_list_service(&self) -> ThreadListService`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_ext_thread_list_service() {
    let _method: fn(&Room) -> ThreadListService = RoomExt::thread_list_service;
    let _ = _method;
}

/// P0.3b-timeline-is-threaded — `Timeline::is_threaded`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/mod.rs` (`pub fn is_threaded`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_is_threaded() {
    fn _shape(timeline: &Timeline) -> bool {
        timeline.is_threaded()
    }
    let _ = _shape;
}

/// Run every threads/relations probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_room_relations();
    let _ = probe_thread_list_service_type();
    probe_room_ext_thread_list_service();
    probe_timeline_is_threaded();
}
