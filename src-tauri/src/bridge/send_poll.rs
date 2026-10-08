//! Desktop bridges for poll start/respond through `Core::command`.

use synara_core::app::send::{MatrixPollRespondResult, MatrixSendPollResult};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn send_poll(
    core: &Core,
    room_id: String,
    question: String,
    answers: Vec<String>,
    max_selections: u32,
    thread_root: Option<String>,
    reply_to: Option<String>,
) -> Result<MatrixSendPollResult, MatrixAuthCommandError> {
    let response = core
        .send_poll(synara_core::core_api::MatrixSendPollRequest {
            room_id,
            question,
            answers,
            max_selections,
            thread_root,
            reply_to,
        })
        .await
        .map_err(map_send_poll_core_error)?;
    Ok(response)
}

pub(crate) async fn poll_respond(
    core: &Core,
    room_id: String,
    poll_event_id: String,
    answer_ids: Vec<String>,
) -> Result<MatrixPollRespondResult, MatrixAuthCommandError> {
    let response = core
        .poll_respond(synara_core::core_api::MatrixPollRespondRequest {
            room_id,
            poll_event_id,
            answer_ids,
        })
        .await
        .map_err(map_poll_respond_core_error)?;
    Ok(response)
}

fn map_send_poll_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    map_poll_core_error(
        error,
        "v-send.3-poll-sdk-failed",
        "The native Matrix poll could not be sent.",
    )
}

fn map_poll_respond_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    map_poll_core_error(
        error,
        "v-send.3-poll-response-sdk-failed",
        "The native Matrix poll response could not be sent.",
    )
}

fn map_poll_core_error(
    error: MatrixIpcError,
    fallback: &'static str,
    unknown_message: &'static str,
) -> MatrixAuthCommandError {
    let diagnostic = error.diagnostic_id.as_deref().unwrap_or(fallback);
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-send.3-poll-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix poll request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new("Unknown", unknown_message, diagnostic),
    }
}
