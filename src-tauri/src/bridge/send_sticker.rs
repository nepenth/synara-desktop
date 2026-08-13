//! Desktop bridge for sticker send through `Core::command`.

use synara_core::app::send::MatrixSendStickerResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_sticker(
    core: &Core,
    room_id: String,
    body: String,
    mxc: String,
    width: Option<u64>,
    height: Option<u64>,
    mimetype: Option<String>,
    size: Option<u64>,
    reply_to: Option<String>,
    thread_root: Option<String>,
) -> Result<MatrixSendStickerResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_send_sticker".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "body": body,
                "mxc": mxc,
                "width": width,
                "height": height,
                "mimetype": mimetype,
                "size": size,
                "replyTo": reply_to,
                "threadRoot": thread_root,
            }),
        })
        .await
        .map_err(map_send_sticker_core_error)?;
    parse_send_sticker_result(response.payload)
}

fn parse_send_sticker_result(
    payload: serde_json::Value,
) -> Result<MatrixSendStickerResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        event_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| send_sticker_response_error())?;
    if wire.status != "sent" {
        return Err(send_sticker_response_error());
    }
    Ok(MatrixSendStickerResult {
        room_id: wire.room_id,
        event_id: wire.event_id,
        status: "sent",
    })
}

fn map_send_sticker_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-send-sticker-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-send-sticker-room-not-found" => {
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
            "The native Matrix sticker could not be sent.",
            diagnostic,
        ),
    }
}

fn send_sticker_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix sticker could not be sent.",
        "v-send-sticker-sdk-failed",
    )
}
