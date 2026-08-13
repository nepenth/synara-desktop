//! Desktop bridge for `matrix_get_room_image_packs` through `Core::command`.

use synara_core::app::account_data::NativeRoomImagePacksSnapshot;
use synara_core::transport::CommandEnvelope;
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

use super::user_image_pack::map_image_pack_core_error;

const ROOM_IMAGE_PACKS_COMMAND: &str = "matrix_get_room_image_packs";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_image_packs(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomImagePacksSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: ROOM_IMAGE_PACKS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "roomId": room_id }),
        })
        .await
        .map_err(map_image_pack_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix image-pack projection is unavailable.",
            "v-send.r-pack-read-fetch-failed",
        )
    })
}
