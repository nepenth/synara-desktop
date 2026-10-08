//! Desktop bridge for `matrix_get_user_image_pack` through `Core::command`.

use synara_core::app::account_data::NativeUserImagePackSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn user_image_pack(
    core: &Core,
) -> Result<NativeUserImagePackSnapshot, MatrixAuthCommandError> {
    let response = core
        .get_user_image_pack()
        .await
        .map_err(map_image_pack_core_error)?;
    Ok(response)
}

pub(crate) fn map_image_pack_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
        _ if error.diagnostic_id.as_deref() == Some("v-send.r-pack-read-room-missing") => {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix image-pack room was not found.",
                "v-send.r-pack-read-room-missing",
            )
        }
        _ => image_pack_unavailable(),
    }
}

fn image_pack_unavailable() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix image-pack projection is unavailable.",
        "v-send.r-pack-read-fetch-failed",
    )
}
