//! Desktop bridges for composer reply-draft commands through `Core::command`.

use synara_core::app::timeline::NativeComposerReplyDraftReadback;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn composer_set_reply_draft(
    core: &Core,
    room_id: String,
    event_id: String,
    start_thread: bool,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let response = core
        .composer_set_reply_draft(synara_core::core_api::MatrixComposerSetReplyDraftRequest {
            room_id,
            event_id,
            start_thread,
        })
        .await
        .map_err(map_composer_core_error)?;
    Ok(response)
}

pub(crate) async fn composer_clear_reply_draft(
    core: &Core,
    room_id: String,
    expected_draft_revision: u64,
    thread_root_event_id: Option<String>,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let response = core
        .composer_clear_reply_draft(
            synara_core::core_api::MatrixComposerClearReplyDraftRequest {
                room_id,
                expected_draft_revision,
                thread_root_event_id,
            },
        )
        .await
        .map_err(map_composer_core_error)?;
    Ok(response)
}

pub(crate) async fn composer_get_reply_draft(
    core: &Core,
    room_id: String,
    thread_root_event_id: Option<String>,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let response = core
        .composer_get_reply_draft(synara_core::core_api::MatrixComposerReplyDraftRoomRequest {
            room_id,
            thread_root_event_id,
        })
        .await
        .map_err(map_composer_core_error)?;
    Ok(response)
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
