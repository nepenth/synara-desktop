//! Desktop AppHandle adapter for the Core image-pack owner.
//!
//! `NativeImagePackOwner` lives in synara-core. This file only maps the
//! existing `matrix-image-packs-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

pub use synara_core::app::account_data::{
    set_global_image_packs, set_room_image_pack, set_user_image_pack, snapshot_global_image_packs,
    snapshot_room_image_packs, snapshot_user_image_pack, NativeGlobalImagePacksSnapshot,
    NativeImagePack, NativeImagePackOwner, NativeImagePackUpdateSignal,
    NativeRoomImagePacksSnapshot, NativeUserImagePackSnapshot, IMAGE_PACKS_UPDATED_EVENT,
};

/// Start the Core owner and emit pack wakeups on the existing Tauri event.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeImagePackOwner, &'static str> {
    NativeImagePackOwner::start(
        client,
        Arc::new(move |signal: NativeImagePackUpdateSignal| {
            let _ = app.emit(IMAGE_PACKS_UPDATED_EVENT, signal);
        }),
        session_generation,
    )
}
