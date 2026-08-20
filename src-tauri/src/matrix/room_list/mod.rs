//! P4.2–P4.4 — Room-list snapshot/delta, membership/unread, tag/recent semantics.
//!
//! SNC-P1-5b: the room-list logic now lives in the shared native core at
//! `crates/synara-core/src/app/room_list`. This module keeps every
//! `crate::matrix::room_list::…` path resolving with **identical behavior** by
//! re-exporting the core items (same pattern as the SNC-P1-5a sync adapter).
//!
//! `tests.rs` and `product_commands.rs` stay here (adapter side): `tests.rs`
//! keeps the desktop room-list suite (15 tests) via `use super::*`, and
//! `product_commands.rs` hosts the Tauri Platform matrix_room_list_* /
//! matrix_invites_snapshot commands (serial product lane, untouchable).
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p4.2-room-list.md`
//! - `docs/matrix-rust-sdk/p4.3-membership-unread.md`
//! - `docs/matrix-rust-sdk/p4.4-room-semantics.md`

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::room_list::{
    contains_bad_word, filter_rooms_by_scope, matrix_room_list_markers, partition_favorite_rooms,
    reconstruct, room_matches_scope, select_rooms_by_scope, select_rooms_in_folder,
    snapshot_from_sync_owner, snapshot_invites, sort_rooms, sort_rooms_in_place,
    InviteAvatarHandles, InviteAvatarSource, NativeInvite, NativeInviteSnapshot,
    NativeInviteTriage, NativeRoomListSnapshot, RoomListBadgeCounts, RoomListDeltaBatch,
    RoomListDeltaOp, RoomListError, RoomListProjection, RoomListScope, RoomListSnapshot,
    RoomListSort, RoomSummaryBuilder, MATRIX_ROOM_LIST_MARKER, MAX_INVITE_AVATAR_HANDLES,
};

#[cfg(test)]
mod tests;
