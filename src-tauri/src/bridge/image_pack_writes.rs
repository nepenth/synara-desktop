//! Desktop bridges for image-pack writes through `Core::command`.

use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::auth::product::MatrixProfileWriteResult;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn set_user_image_pack(
    core: &Core,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    dispatch_write(
        core,
        "matrix_set_user_image_pack",
        serde_json::json!({ "content": content }),
    )
    .await
}

pub(crate) async fn set_global_image_packs(
    core: &Core,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    dispatch_write(
        core,
        "matrix_set_global_image_packs",
        serde_json::json!({ "content": content }),
    )
    .await
}

pub(crate) async fn set_room_image_pack(
    core: &Core,
    room_id: String,
    state_key: String,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    dispatch_write(
        core,
        "matrix_set_room_image_pack",
        serde_json::json!({
            "roomId": room_id,
            "stateKey": state_key,
            "content": content,
        }),
    )
    .await
}

async fn dispatch_write(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map_err(map_image_pack_write_core_error)?;
    Ok(MatrixProfileWriteResult { status: "ok" })
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
