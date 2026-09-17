//! Product-owned Tauri command for live homeserver message search.

use super::*;

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_message_search(
    core: State<'_, Arc<synara_core::Core>>,
    term: String,
    next_token: Option<String>,
    rooms: Option<Vec<String>>,
    senders: Option<Vec<String>>,
    order: Option<String>,
    listing_kind: Option<String>,
    from_ts: Option<u64>,
    to_ts: Option<u64>,
) -> Result<synara_core::app::search::MatrixMessageSearchResult, MatrixAuthCommandError> {
    crate::bridge::message_search::message_search(
        core.inner().as_ref(),
        term,
        next_token,
        rooms,
        senders,
        order,
        listing_kind,
        from_ts,
        to_ts,
    )
    .await
}
