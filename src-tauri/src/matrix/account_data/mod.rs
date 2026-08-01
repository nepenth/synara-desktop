//! P6.7 account-data foundation + live Synara account-data owners.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.7-account-data.md`
//! Product verticals: `docs/matrix-rust-sdk/v-rooms-5-mdirect.md`,
//! `docs/matrix-rust-sdk/v-timeline-full-replacement-contract.md` (later/notes).

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod image_packs;
mod index;
pub mod later;
pub mod live;
pub mod room_notes;

pub use error::AccountDataError;
pub use image_packs::{
    snapshot_global_image_packs, snapshot_room_image_packs, snapshot_user_image_pack,
    NativeGlobalImagePacksSnapshot, NativeImagePack, NativeRoomImagePacksSnapshot,
    NativeUserImagePackSnapshot,
};
pub use index::{
    AccountDataEntry, AccountDataIndex, MAX_CONTENT_FIELDS, MAX_GLOBAL_TYPES, MAX_KEY_LEN,
    MAX_ROOMS_WITH_ACCOUNT_DATA, MAX_ROOM_TYPES, MAX_VALUE_LEN, TYPE_DIRECT, TYPE_FULLY_READ,
    TYPE_IGNORED_USER_LIST, TYPE_PUSH_RULES, TYPE_TAG,
};
pub use later::{
    clear_completed_later_live, complete_later_item_live, mark_later_reminded_live, snapshot_later,
    snooze_later_item_live, upsert_later_item, NativeLaterSnapshot, SynaraLaterContent,
    SynaraLaterItem, SynaraLaterItemKind,
};
pub use live::{
    add_room_to_mdirect, remove_room_from_mdirect, snapshot_mdirect, NativeMDirectMutationResult,
    NativeMDirectSnapshot,
};
pub use room_notes::{
    complete_room_todo_item_live, delete_room_note_item_live, move_room_todo_item_live,
    snapshot_room_notes, upsert_room_note_item, NativeRoomNotesSnapshot, RoomNoteMoveDirection,
    SynaraRoomNoteItem, SynaraRoomNoteItemKind, SynaraRoomNotesContent,
};

/// Static marker for link / schema smoke.
pub const MATRIX_ACCOUNT_DATA_MARKER: &str = "matrix-account-data-p6.7";

/// Touch account-data paths so they remain linked in non-test builds.
pub fn matrix_account_data_markers() -> &'static str {
    let idx = AccountDataIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(TYPE_FULLY_READ, "m.fully_read");
    debug_assert_eq!(MATRIX_ACCOUNT_DATA_MARKER, "matrix-account-data-p6.7");
    MATRIX_ACCOUNT_DATA_MARKER
}

#[cfg(test)]
mod tests;
