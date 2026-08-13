//! Desktop bridge for `matrix_get_global_image_packs` through `Core::command`.

use synara_core::app::account_data::NativeGlobalImagePacksSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const GLOBAL_IMAGE_PACKS_COMMAND: &str = "matrix_get_global_image_packs";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn global_image_packs(
    core: &Core,
) -> Result<NativeGlobalImagePacksSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: GLOBAL_IMAGE_PACKS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_global_image_packs_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix image-pack projection is unavailable.",
            "v-send.r-pack-read-fetch-failed",
        )
    })
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
