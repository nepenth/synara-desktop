//! Desktop bridge for composer text send through `Core::command`.

use synara_core::app::send::MatrixSendTextResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_text(
    core: &Core,
    room_id: String,
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: Option<bool>,
    reply_to: Option<String>,
    thread_root: Option<String>,
    txn_id: Option<String>,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    let response = core
        .send_text(synara_core::core_api::MatrixSendTextRequest {
            room_id,
            body,
            msg_type,
            formatted_body,
            mention_user_ids,
            mention_room,
            reply_to,
            thread_root,
            txn_id,
        })
        .await
        .map_err(map_send_text_core_error)?;
    Ok(response)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn edit_message(
    core: &Core,
    room_id: String,
    event_id: String,
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: Option<bool>,
    txn_id: Option<String>,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    let response = core
        .edit_message(synara_core::core_api::MatrixEditMessageRequest {
            room_id,
            event_id,
            body,
            msg_type,
            formatted_body,
            mention_user_ids,
            mention_room,
            txn_id,
        })
        .await
        .map_err(map_edit_message_core_error)?;
    Ok(response)
}

fn map_send_text_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("d0.4-send-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "d0.4-send-room-not-found" | "v-send.r-edit-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix send request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix message could not be sent.",
            diagnostic,
        ),
    }
}

fn map_edit_message_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-send.r-edit-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-send.r-edit-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix send request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix message edit could not be sent.",
            diagnostic,
        ),
    }
}

pub(crate) async fn discard_local_echo(
    core: &Core,
    room_id: String,
    transaction_id: String,
) -> Result<bool, MatrixAuthCommandError> {
    let response = core
        .local_echo_discard(synara_core::core_api::MatrixLocalEchoRequest {
            room_id,
            transaction_id,
        })
        .await
        .map_err(|error| map_local_echo_error(error, "discard"))?;
    Ok(response.aborted)
}

pub(crate) async fn retry_local_echo(
    core: &Core,
    room_id: String,
    transaction_id: String,
) -> Result<(), MatrixAuthCommandError> {
    let response = core
        .local_echo_retry(synara_core::core_api::MatrixLocalEchoRequest {
            room_id,
            transaction_id,
        })
        .await
        .map_err(|error| map_local_echo_error(error, "retry"))?;
    if response.status != "retrying" {
        return Err(local_echo_response_error("retry"));
    }
    Ok(())
}

fn map_local_echo_error(error: MatrixIpcError, action: &'static str) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("d0.4-send-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            diagnostic,
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix send request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new("Unknown", local_echo_failure_message(action), diagnostic),
    }
}

fn local_echo_failure_message(action: &'static str) -> &'static str {
    if action == "discard" {
        "The unsent message could not be discarded."
    } else {
        "The unsent message could not be retried."
    }
}

fn local_echo_response_error(action: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        local_echo_failure_message(action),
        "d0.4-send-sdk-failed",
    )
}
