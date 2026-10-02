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

mod activity_recovery;
mod counts;
mod delta;
mod dm_avatar;
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
    client_total_unread_notifications, room_unread_presentation, sdk_joined_notification_checksum,
    synara_joined_unread_presentation_sum, RoomListBadgeCounts, RoomUnreadMembership,
    RoomUnreadPresentationDto, SdkUnreadChecksumRoom, SynaraUnreadChecksumRoom,
};
pub use delta::{RoomListDeltaBatch, RoomListDeltaOp, RoomListSnapshot};
pub use dm_avatar::{dm_avatar_source, select_dm_avatar_source, DmAvatarSourceKind};
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
    last_message_event_is_agent_approval, last_message_event_is_agent_approval_str,
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
