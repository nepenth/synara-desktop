//! Desktop bridges for poll start/respond through `Core::command`.

use synara_core::app::send::{MatrixPollRespondResult, MatrixSendPollResult};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

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
        .command(CommandEnvelope {
            command: "matrix_send_poll".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "question": question,
                "answers": answers,
                "maxSelections": max_selections,
                "threadRoot": thread_root,
                "replyTo": reply_to,
            }),
        })
        .await
        .map_err(map_send_poll_core_error)?;
    parse_send_poll_result(response.payload)
}

pub(crate) async fn poll_respond(
    core: &Core,
    room_id: String,
    poll_event_id: String,
    answer_ids: Vec<String>,
) -> Result<MatrixPollRespondResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_poll_respond".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "pollEventId": poll_event_id,
                "answerIds": answer_ids,
            }),
        })
        .await
        .map_err(map_poll_respond_core_error)?;
    parse_poll_respond_result(response.payload)
}

fn parse_send_poll_result(
    payload: serde_json::Value,
) -> Result<MatrixSendPollResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        event_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| send_poll_response_error())?;
    if wire.status != "sent" {
        return Err(send_poll_response_error());
    }
    Ok(MatrixSendPollResult {
        room_id: wire.room_id,
        event_id: wire.event_id,
        status: "sent",
    })
}

fn parse_poll_respond_result(
    payload: serde_json::Value,
) -> Result<MatrixPollRespondResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        poll_event_id: String,
        event_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| poll_respond_response_error())?;
    if wire.status != "sent" {
        return Err(poll_respond_response_error());
    }
    Ok(MatrixPollRespondResult {
        room_id: wire.room_id,
        poll_event_id: wire.poll_event_id,
        event_id: wire.event_id,
        status: "sent",
    })
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

fn send_poll_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix poll could not be sent.",
        "v-send.3-poll-sdk-failed",
    )
}

fn poll_respond_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix poll response could not be sent.",
        "v-send.3-poll-response-sdk-failed",
    )
}
