//! Desktop bridge for `matrix_message_search` through `Core::command`.

use synara_core::app::search::MatrixMessageSearchResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub(crate) async fn message_search(
    core: &Core,
    term: String,
    next_token: Option<String>,
    rooms: Option<Vec<String>>,
    senders: Option<Vec<String>>,
    order: Option<String>,
    listing_kind: Option<String>,
    from_ts: Option<u64>,
    to_ts: Option<u64>,
) -> Result<MatrixMessageSearchResult, MatrixAuthCommandError> {
    core.message_search(synara_core::core_api::MatrixMessageSearchRequest {
        term,
        next_token,
        rooms,
        senders,
        order,
        listing_kind,
        from_ts,
        to_ts,
    })
    .await
    .map_err(map_message_search_core_error)
}

fn map_message_search_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-search.sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "p2-message-search-no-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native message-search request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native message search is unavailable.",
            diagnostic,
        ),
    }
}
