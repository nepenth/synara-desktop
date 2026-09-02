//! P6.7 account-data foundation (SNC-P1 core harness).
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.7-account-data.md`
//! Product verticals: `docs/matrix-rust-sdk/v-rooms-5-mdirect.md`,
//! `docs/matrix-rust-sdk/v-timeline-full-replacement-contract.md` (later/notes).
//! Live image-pack snapshot/set/owner, m.direct, later, and room-notes Client
//! RMW live here.

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod image_packs;
mod image_packs_live;
mod index;
mod later;
mod later_live;
mod mdirect;
mod mdirect_live;
mod room_notes;
mod room_notes_live;

pub use error::AccountDataError;
pub use image_packs::{
    is_image_pack_account_data_type, is_image_pack_room_state_type, pack_from_account_data,
    set_global_image_packs_content_guard, set_room_image_pack_content_guard,
    set_user_image_pack_content_guard, EmoteRoomsContent, NativeGlobalImagePacksSnapshot,
    NativeImagePack, NativeRoomImagePacksSnapshot, NativeUserImagePackSnapshot,
    EMOTE_ROOMS_EVENT_TYPE, IMAGE_PACKS_UPDATED_EVENT, ROOM_EMOTES_EVENT_TYPE,
    USER_EMOTES_EVENT_TYPE,
};
pub use image_packs_live::{
    set_global_image_packs, set_room_image_pack, set_user_image_pack, snapshot_global_image_packs,
    snapshot_room_image_packs, snapshot_user_image_pack, ImagePackUpdateEmit, NativeImagePackOwner,
    NativeImagePackUpdateSignal,
};
pub use index::{
    AccountDataEntry, AccountDataIndex, MAX_CONTENT_FIELDS, MAX_GLOBAL_TYPES, MAX_KEY_LEN,
    MAX_ROOMS_WITH_ACCOUNT_DATA, MAX_ROOM_TYPES, MAX_VALUE_LEN, TYPE_DIRECT, TYPE_FULLY_READ,
    TYPE_IGNORED_USER_LIST, TYPE_PUSH_RULES, TYPE_TAG,
};
pub use later::{
    clear_completed_later_items, complete_later_item, mark_later_reminded, normalize_later_content,
    normalize_later_item, put_later_item, snooze_later_item, NativeLaterSnapshot,
    SynaraLaterContent, SynaraLaterItem, SynaraLaterItemKind, LATER_ACCOUNT_DATA_VERSION,
    LATER_EVENT_TYPE,
};
pub use later_live::{
    clear_completed_later_live, complete_later_item_live, later_timestamp_or_now,
    mark_later_reminded_live, snapshot_later, snooze_later_item_live, upsert_later_item,
};
pub use mdirect::{
    apply_add_mdirect_room, apply_remove_mdirect_room, snapshot_from_mdirect_rooms, MDirectRooms,
    NativeMDirectMutationResult, NativeMDirectSnapshot,
};
pub use mdirect_live::{add_room_to_mdirect, remove_room_from_mdirect, snapshot_mdirect};
pub use room_notes::{
    complete_room_todo_item, limit_text, move_room_todo_item, normalize_room_note_item,
    normalize_room_notes_content, normalize_room_notes_content_checked, put_room_note_item,
    remove_room_note_item, validate_room_note_mutation_target, validate_room_notes_content_size,
    NativeRoomNotesSnapshot, RoomNoteMoveDirection, SynaraRoomNoteItem, SynaraRoomNoteItemKind,
    SynaraRoomNotesContent, SynaraRoomNotesRoom, MAX_MESSAGE_BODY_LENGTH, MAX_NOTE_BODY_LENGTH,
    MAX_NOTE_ID_LENGTH, MAX_ROOM_ID_BYTES, MAX_ROOM_NOTES_CONTENT_BYTES, MAX_SENDER_LENGTH,
    ROOM_NOTES_ACCOUNT_DATA_VERSION, ROOM_NOTES_EVENT_TYPE,
};
pub use room_notes_live::{
    complete_room_todo_item_live, delete_room_note_item_live, move_room_todo_item_live,
    room_notes_now_ms, snapshot_room_notes, upsert_room_note_item,
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
