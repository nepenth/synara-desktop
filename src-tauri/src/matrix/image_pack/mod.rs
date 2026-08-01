//! V-SEND.R-PACK-READ native pack-read projection.
//!
//! Read-only projection of the Ponies emoji/sticker pack surface
//! (`im.ponies.user_emotes`, `im.ponies.emote_rooms`, `im.ponies.room_emotes`)
//! into a serializable DTO for the desktop frontend. This slice is strictly
//! read-only and fail-closed: on a native logged-in session, absence or failure
//! of any `matrix_get_*_image_packs` command is terminal and must not fall back
//! to the live `matrix-js-sdk` account-data/state-event reads.
//!
//! Design note: `docs/matrix-rust-sdk/v-send-pack-read-residual.md`.
//! Pack **write** (add/remove/enable/update) and media **upload** are separate
//! residuals (V-SEND.R-PACK-WRITE / V-SEND.R-PACK-UPLOAD) and are out of scope.

#![allow(dead_code)]

mod projection;

pub use projection::{
    snapshot_global_image_packs, snapshot_room_image_packs, snapshot_user_image_pack,
    NativeImagePack, NativeImagePackAddress, NativeImagePackImage, NativeImagePackMeta,
    NativeImagePackSnapshot,
};

/// Static marker for link / schema smoke.
pub const MATRIX_IMAGE_PACK_MARKER: &str = "matrix-image-pack-v-send-r-pack-read";

/// Touch image-pack paths so they remain linked in non-test builds.
pub fn matrix_image_pack_markers() -> &'static str {
    let _ = NativeImagePackMeta::default();
    let _addr = NativeImagePackAddress {
        room_id: String::new(),
        state_key: String::new(),
    };
    let _img = NativeImagePackImage {
        url: String::new(),
        body: None,
        usage: None,
        info: None,
    };
    let _pack = NativeImagePack {
        id: String::new(),
        deleted: false,
        address: Some(_addr),
        meta: NativeImagePackMeta::default(),
        images: std::collections::BTreeMap::new(),
    };
    let _ = _img;
    let _ = _pack;
    MATRIX_IMAGE_PACK_MARKER
}
