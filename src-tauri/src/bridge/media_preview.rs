//! Desktop bridge for `matrix_media_preview` through `Core::command`.

use synara_core::app::media::MatrixMediaPreviewSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const COMMAND: &str = "matrix_media_preview";

pub(crate) async fn media_preview(
    core: &Core,
    room_id: String,
    session_generation: u64,
    url: String,
    ts: Option<u64>,
) -> Result<MatrixMediaPreviewSnapshot, MatrixAuthCommandError> {
    let mut payload = serde_json::json!({
        "roomId": room_id,
        "sessionGeneration": session_generation,
        "url": url,
    });
    if let Some(ts) = ts {
        payload
            .as_object_mut()
            .expect("preview payload is an object")
            .insert("ts".into(), serde_json::json!(ts));
    }
    let response = core
        .command(CommandEnvelope {
            command: COMMAND.to_owned(),
            session_generation,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_media_preview_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| preview_response_error())
}

fn map_media_preview_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-send.r-media-preview-unavailable");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-send.r-media-preview-requires-session",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "Forbidden",
            "The native Matrix media preview session is stale.",
            "v-send.r-media-preview-stale-generation",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-send.r-media-preview-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                "v-send.r-media-preview-url-invalid" => {
                    ("InvalidRequest", "The URL cannot be previewed.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix media preview request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix URL preview is unavailable.",
            diagnostic,
        ),
    }
}

fn preview_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix URL preview is unavailable.",
        "v-send.r-media-preview-unavailable",
    )
}
