//! Desktop bridges for image-pack writes through `Core::command`.

use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::auth::product::MatrixProfileWriteResult;

pub(crate) async fn set_user_image_pack(
    core: &Core,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    core.set_user_image_pack(synara_core::core_api::MatrixSetImagePackContentRequest { content })
        .await
        .map(|_| MatrixProfileWriteResult {
            status: synara_core::dto::WriteAck::Ok,
        })
        .map_err(map_image_pack_write_core_error)
}

pub(crate) async fn set_global_image_packs(
    core: &Core,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    core.set_global_image_packs(synara_core::core_api::MatrixSetImagePackContentRequest { content })
        .await
        .map(|_| MatrixProfileWriteResult {
            status: synara_core::dto::WriteAck::Ok,
        })
        .map_err(map_image_pack_write_core_error)
}

pub(crate) async fn set_room_image_pack(
    core: &Core,
    room_id: String,
    state_key: String,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    core.set_room_image_pack(synara_core::core_api::MatrixSetRoomImagePackRequest {
        room_id,
        state_key,
        content,
    })
    .await
    .map(|_| MatrixProfileWriteResult {
        status: synara_core::dto::WriteAck::Ok,
    })
    .map_err(map_image_pack_write_core_error)
}

fn map_image_pack_write_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-send.r-pack-read-no-user",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix image-pack write is invalid.",
            "v-send.r-pack-write-invalid-content",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix image-pack write is unavailable.",
            "v-send.r-pack-write-set-failed",
        ),
    }
}
