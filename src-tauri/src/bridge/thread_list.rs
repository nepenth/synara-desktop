//! Desktop bridge for `matrix_thread_list` through `Core::command`.

use synara_core::app::threads::NativeThreadListSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const THREAD_LIST_COMMAND: &str = "matrix_thread_list";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn thread_list(
    core: &Core,
    room_id: String,
    action: String,
) -> Result<NativeThreadListSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: THREAD_LIST_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "action": action,
            }),
        })
        .await
        .map_err(map_thread_list_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| thread_list_response_error())
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

fn thread_list_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix thread list request is invalid.",
        "v-thread-list-paginate-failed",
    )
}
