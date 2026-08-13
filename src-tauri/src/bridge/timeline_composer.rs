//! Desktop bridges for composer reply-draft commands through `Core::command`.

use synara_core::app::timeline::NativeComposerReplyDraftReadback;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const COMPOSER_SET_COMMAND: &str = "matrix_composer_set_reply_draft";
const COMPOSER_CLEAR_COMMAND: &str = "matrix_composer_clear_reply_draft";
const COMPOSER_GET_COMMAND: &str = "matrix_composer_get_reply_draft";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn composer_set_reply_draft(
    core: &Core,
    room_id: String,
    event_id: String,
    start_thread: bool,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: COMPOSER_SET_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "startThread": start_thread,
            }),
        })
        .await
        .map_err(map_composer_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| composer_response_error())
}

pub(crate) async fn composer_clear_reply_draft(
    core: &Core,
    room_id: String,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    dispatch_room_draft(core, COMPOSER_CLEAR_COMMAND, room_id).await
}

pub(crate) async fn composer_get_reply_draft(
    core: &Core,
    room_id: String,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    dispatch_room_draft(core, COMPOSER_GET_COMMAND, room_id).await
}

async fn dispatch_room_draft(
    core: &Core,
    command: &str,
    room_id: String,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "roomId": room_id }),
        })
        .await
        .map_err(map_composer_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| composer_response_error())
}

fn map_composer_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-reply-draft-invalid-event-id");
            let (code, message) = if diagnostic == "v-timeline-reply-draft-room-not-found" {
                ("NotFound", "The native Matrix room is not available.")
            } else {
                (
                    "InvalidRequest",
                    "The native Matrix timeline action request is invalid.",
                )
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-reply-draft-event-unavailable");
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline action request is invalid.",
                diagnostic,
            )
        }
    }
}

fn composer_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline action request is invalid.",
        "v-timeline-reply-draft-event-unavailable",
    )
}
