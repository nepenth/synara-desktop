//! Room create/join/leave/invite/member/profile/power compile-only probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::room::RoomMember;
use matrix_sdk::ruma::api::client::room::create_room;
use matrix_sdk::ruma::api::client::state::send_state_event;
use matrix_sdk::ruma::{Int, RoomId, UserId};
use matrix_sdk::{Account, Client, Room, RoomMemberships};

/// P0.3b-client-create-room — `Client::create_room`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn create_room`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_create_room() {
    async fn _shape(
        client: &Client,
        request: create_room::v3::Request,
    ) -> matrix_sdk::Result<Room> {
        client.create_room(request).await
    }
    let _ = _shape;
}

/// P0.3b-client-join-room-by-id — `Client::join_room_by_id`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn join_room_by_id`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_join_room_by_id() {
    async fn _shape(client: &Client, room_id: &RoomId) -> matrix_sdk::Result<Room> {
        client.join_room_by_id(room_id).await
    }
    let _ = _shape;
}

/// P0.3b-room-leave — `Room::leave`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn leave`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_leave() {
    async fn _shape(room: &Room) -> matrix_sdk::Result<()> {
        room.leave().await
    }
    let _ = _shape;
}

/// P0.3b-room-invite-user-by-id — `Room::invite_user_by_id`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn invite_user_by_id`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_invite_user_by_id() {
    async fn _shape(room: &Room, user_id: &UserId) -> matrix_sdk::Result<()> {
        room.invite_user_by_id(user_id).await
    }
    let _ = _shape;
}

/// P0.3b-room-members — `Room::members`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn members`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_members() {
    async fn _shape(
        room: &Room,
        memberships: RoomMemberships,
    ) -> matrix_sdk::Result<Vec<RoomMember>> {
        room.members(memberships).await
    }
    let _ = _shape;
}

/// P0.3b-room-set-name — `Room::set_name`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn set_name`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_set_name() {
    async fn _shape(
        room: &Room,
        name: String,
    ) -> matrix_sdk::Result<send_state_event::v3::Response> {
        room.set_name(name).await
    }
    let _ = _shape;
}

/// P0.3b-room-update-power-levels — `Room::update_power_levels`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn update_power_levels`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_update_power_levels() {
    async fn _shape(
        room: &Room,
        updates: Vec<(&UserId, Int)>,
    ) -> matrix_sdk::Result<send_state_event::v3::Response> {
        room.update_power_levels(updates).await
    }
    let _ = _shape;
}

/// P0.3b-room-ban-user — `Room::ban_user`.
///
/// Source: `crates/matrix-sdk/src/room/mod.rs` (`pub async fn ban_user`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_room_ban_user() {
    async fn _shape(room: &Room, user_id: &UserId, reason: Option<&str>) -> matrix_sdk::Result<()> {
        room.ban_user(user_id, reason).await
    }
    let _ = _shape;
}

/// P0.3b-account-type — `matrix_sdk::Account` is a public type.
///
/// Source: `crates/matrix-sdk/src/account.rs` (`pub struct Account`) and
/// crate-root re-export.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_account_type() -> &'static str {
    std::any::type_name::<Account>()
}

/// P0.3b-client-account — `Client::account() -> Account`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn account`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_account() {
    fn _shape(client: &Client) -> Account {
        client.account()
    }
    let _ = _shape;
}

/// P0.3b-account-get-display-name — `Account::get_display_name`.
///
/// Source: `crates/matrix-sdk/src/account.rs` (`pub async fn get_display_name`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_account_get_display_name() {
    async fn _shape(account: &Account) -> matrix_sdk::Result<Option<String>> {
        account.get_display_name().await
    }
    let _ = _shape;
}

/// Run every room-ops probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_client_create_room();
    probe_client_join_room_by_id();
    probe_room_leave();
    probe_room_invite_user_by_id();
    probe_room_members();
    probe_room_set_name();
    probe_room_update_power_levels();
    probe_room_ban_user();
    let _ = probe_account_type();
    probe_client_account();
    probe_account_get_display_name();
}
