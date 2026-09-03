//! P4.2–P4.4 — Room-list snapshot/delta, membership/unread, tag/recent semantics.
//!
//! SNC-P1-5b: moved from src-tauri `matrix/room_list` into the shared native
//! core (`crates/synara-core/src/app/room_list`). src-tauri's
//! `matrix/room_list/mod.rs` is now an adapter that re-exports this module;
//! `room_list/tests.rs` and `product_commands.rs` stay in the desktop shell.
//!
//! Deterministic projection of ordered room summaries for the partial path:
//! - product DTOs only ([`RoomSummary`]) — no SDK Room/VectorDiff on the wire
//! - ordered delta ops with monotonic sequence + session generation
//! - gap / stale generation → resync (full snapshot reset)
//! - P4.3: scope filters (joined/invites/unread/mentions/direct) + badge counts
//! - P4.4: favorite / low-priority / folder filters + name/recent-activity sorts
//!
//! D0.2 adds a production snapshot projection backed only by matrix-sdk-ui's
//! room-list service. There is no JS room-list fallback while a native session
//! is live and no dual-backend sync.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p4.2-room-list.md`
//! - `docs/matrix-rust-sdk/p4.3-membership-unread.md`
//! - `docs/matrix-rust-sdk/p4.4-room-semantics.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod activity_recovery;
mod counts;
mod delta;
mod error;
mod filters;
mod invite_avatars;
mod invites;
mod last_message;
mod live;
mod projection;
mod sort;
mod summary;

pub use crate::dto::RoomEncryptionStatus;
pub use activity_recovery::{room_activity_recovery_required, RoomActivityPreviousState};
pub use counts::{
    room_unread_presentation, RoomListBadgeCounts, RoomUnreadMembership, RoomUnreadPresentationDto,
};
pub use delta::{RoomListDeltaBatch, RoomListDeltaOp, RoomListSnapshot};
pub use error::RoomListError;
pub use filters::{
    filter_rooms_by_scope, partition_favorite_rooms, room_matches_scope, select_rooms_by_scope,
    select_rooms_in_folder, RoomListScope,
};
pub use invite_avatars::{InviteAvatarHandles, InviteAvatarSource, MAX_INVITE_AVATAR_HANDLES};
pub use invites::{
    contains_bad_word, snapshot_invites, NativeInvite, NativeInviteSnapshot, NativeInviteTriage,
};
pub use last_message::{
    last_message_preview_from_event_json, last_message_preview_from_event_json_str,
    last_message_preview_from_invite, sanitize_last_message_preview,
};
pub use live::{
    snapshot_from_sync_owner, NativeRoomListOwner, NativeRoomListSnapshot,
    NativeRoomListUpdateSignal, RoomListUpdateEmit,
};
pub use projection::{reconstruct, RoomListProjection};
pub use sort::{sort_rooms, sort_rooms_in_place, RoomListSort};
pub use summary::RoomSummaryBuilder;

/// Static marker for link / schema smoke.
pub const MATRIX_ROOM_LIST_MARKER: &str =
    "matrix-room-list-p4.2+membership-unread-p4.3+semantics-p4.4";

/// Touch room-list foundation paths so they remain linked in non-test builds.
pub fn matrix_room_list_markers() -> &'static str {
    let mut proj = RoomListProjection::new(1);
    let snap = RoomListSnapshot::empty(1);
    let _ = proj.apply_snapshot(snap);
    let _scopes = RoomListScope::ALL.len();
    let _counts = RoomListBadgeCounts::from_rooms(&[]);
    let _sort = RoomListSort::RecentActivity.as_str();
    debug_assert!(proj.is_empty());
    debug_assert_eq!(proj.last_sequence(), 0);
    debug_assert_eq!(RoomListDeltaOp::Clear.op_name(), "clear");
    debug_assert_eq!(_scopes, 8);
    debug_assert_eq!(_counts.joined, 0);
    debug_assert_eq!(_sort, "recent_activity");
    debug_assert_eq!(
        MATRIX_ROOM_LIST_MARKER,
        "matrix-room-list-p4.2+membership-unread-p4.3+semantics-p4.4"
    );
    MATRIX_ROOM_LIST_MARKER
}
