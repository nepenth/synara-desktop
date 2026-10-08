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

use std::sync::Arc;

use serde::Serialize;
use synara_core::app::room_list::{
    NativeRoomListOwner, NativeRoomListUpdateSignal, RoomListUpdateEmit,
};
use synara_core::app::sync::SyncServiceOwner;
use tauri::{AppHandle, Emitter, Runtime};

/// Renderer wake-up for every native list derived from sync: rooms, invites,
/// direct-message map, space parents, Later and room notes. The payload has
/// no room ids; the renderer re-reads the existing snapshot commands.
pub const MATRIX_ROOM_LIST_UPDATED_EVENT: &str = "matrix-room-list-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomListUpdatedPayload {
    pub session_generation: u64,
    pub revision: u64,
}

impl From<NativeRoomListUpdateSignal> for RoomListUpdatedPayload {
    fn from(signal: NativeRoomListUpdateSignal) -> Self {
        Self {
            session_generation: signal.session_generation,
            revision: signal.revision,
        }
    }
}

/// Main-window-only sink; widget webviews never render room lists.
pub fn room_list_update_emit<R: Runtime>(app: AppHandle<R>) -> RoomListUpdateEmit {
    Arc::new(move |signal| {
        let _ = app.emit_to(
            crate::desktop::MAIN_WINDOW_LABEL,
            MATRIX_ROOM_LIST_UPDATED_EVENT,
            RoomListUpdatedPayload::from(signal),
        );
    })
}

/// Session-scoped owner. It lives in the installed session, so logout or a
/// generation change drops it, which aborts the task and its handlers.
pub fn start_room_list_live(sync: &Arc<SyncServiceOwner>, app: AppHandle) -> NativeRoomListOwner {
    NativeRoomListOwner::start(sync, room_list_update_emit(app))
}

#[cfg(test)]
mod tests;
