//! Desktop bridge for `matrix_thread_list` through `Core::command`.

use synara_core::app::threads::NativeThreadListSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn thread_list(
    core: &Core,
    room_id: String,
    action: String,
) -> Result<NativeThreadListSnapshot, MatrixAuthCommandError> {
    let response = core
        .thread_list(synara_core::core_api::MatrixThreadListRequest { room_id, action })
        .await
        .map_err(map_thread_list_core_error)?;
    Ok(response)
}

fn map_thread_list_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "p2-thread-list-no-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-thread-list-invalid-action");
            let (code, message) = if diagnostic == "v-thread-list-room-not-found" {
                ("NotFound", "The native Matrix room is not available.")
            } else {
                (
                    "InvalidRequest",
                    "The native Matrix thread list request is invalid.",
                )
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix thread list request is invalid.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-thread-list-paginate-failed"),
        ),
    }
}
