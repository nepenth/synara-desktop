//! Desktop room-list owner projections and regression fixtures.

#[cfg(test)]
pub use synara_core::app::room_list::{
    partition_favorite_rooms, reconstruct, room_matches_scope, select_rooms_by_scope,
    select_rooms_in_folder, sort_rooms, RoomListBadgeCounts, RoomListDeltaBatch, RoomListDeltaOp,
    RoomListProjection, RoomListScope, RoomListSnapshot, RoomListSort, RoomSummaryBuilder,
    MAX_INVITE_AVATAR_HANDLES,
};

pub use synara_core::app::room_list::{
    InviteAvatarHandles, InviteAvatarSource, NativeInviteSnapshot, NativeRoomListSnapshot,
};

#[cfg(test)]
mod tests;
