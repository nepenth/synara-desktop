//! Product-owned Tauri command for live homeserver message search.

use super::*;

#[tauri::command]
pub async fn matrix_message_search(
    core: State<'_, Arc<synara_core::Core>>,
    term: String,
    next_token: Option<String>,
    rooms: Option<Vec<String>>,
    senders: Option<Vec<String>>,
    order: Option<String>,
) -> Result<synara_core::app::search::MatrixMessageSearchResult, MatrixAuthCommandError> {
    crate::bridge::message_search::message_search(
        core.inner().as_ref(),
        term,
        next_token,
        rooms,
        senders,
        order,
    )
    .await
}
