//! Desktop bridge for composer text send through `Core::command`.

use synara_core::app::send::MatrixSendTextResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

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
        .command(CommandEnvelope {
            command: "matrix_send_text".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "body": body,
                "msgType": msg_type,
                "formattedBody": formatted_body,
                "mentionUserIds": mention_user_ids,
                "mentionRoom": mention_room,
                "replyTo": reply_to,
                "threadRoot": thread_root,
                "txnId": txn_id,
            }),
        })
        .await
        .map_err(map_send_text_core_error)?;
    parse_send_text_result(response.payload)
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
        .command(CommandEnvelope {
            command: "matrix_edit_message".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "body": body,
                "msgType": msg_type,
                "formattedBody": formatted_body,
                "mentionUserIds": mention_user_ids,
                "mentionRoom": mention_room,
                "txnId": txn_id,
            }),
        })
        .await
        .map_err(map_edit_message_core_error)?;
    parse_send_text_result(response.payload)
}

fn parse_send_text_result(
    payload: serde_json::Value,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        event_id: String,
        local_txn_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| send_text_response_error())?;
    if wire.status != "sent" {
        return Err(send_text_response_error());
    }
    Ok(MatrixSendTextResult {
        room_id: wire.room_id,
        event_id: wire.event_id,
        local_txn_id: wire.local_txn_id,
        status: "sent",
    })
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

fn send_text_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix message could not be sent.",
        "d0.4-send-sdk-failed",
    )
}
