//! Desktop bridge for `matrix_media_preview` through `Core::command`.

use synara_core::app::media::MatrixMediaPreviewSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn media_preview(
    core: &Core,
    room_id: String,
    session_generation: u64,
    url: String,
    ts: Option<u64>,
) -> Result<MatrixMediaPreviewSnapshot, MatrixAuthCommandError> {
    core.media_preview(synara_core::core_api::MatrixMediaPreviewRequest {
        room_id,
        session_generation,
        url,
        ts,
    })
    .await
    .map_err(map_media_preview_core_error)
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
