//! Desktop bridge for `matrix_get_global_image_packs` through `Core::command`.

use synara_core::app::account_data::NativeGlobalImagePacksSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn global_image_packs(
    core: &Core,
) -> Result<NativeGlobalImagePacksSnapshot, MatrixAuthCommandError> {
    let response = core
        .get_global_image_packs()
        .await
        .map_err(map_global_image_packs_core_error)?;
    Ok(response)
}

fn map_global_image_packs_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-send.r-pack-read-no-user",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix image-pack request is invalid.",
            "v-send.r-pack-read-invalid-room",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix image-pack projection is unavailable.",
            "v-send.r-pack-read-fetch-failed",
        ),
    }
}
