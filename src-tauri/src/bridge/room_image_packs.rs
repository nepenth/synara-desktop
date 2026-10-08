//! Desktop bridge for `matrix_get_room_image_packs` through `Core::command`.

use synara_core::app::account_data::NativeRoomImagePacksSnapshot;
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

use super::user_image_pack::map_image_pack_core_error;

pub(crate) async fn room_image_packs(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomImagePacksSnapshot, MatrixAuthCommandError> {
    let response = core
        .get_room_image_packs(synara_core::core_api::MatrixGetRoomImagePacksRequest { room_id })
        .await
        .map_err(map_image_pack_core_error)?;
    Ok(response)
}
