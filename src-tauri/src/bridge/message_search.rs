//! Desktop bridge for `matrix_message_search` through `Core::command`.

use synara_core::app::search::MatrixMessageSearchResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn message_search(
    core: &Core,
    term: String,
    next_token: Option<String>,
    rooms: Option<Vec<String>>,
    senders: Option<Vec<String>>,
    order: Option<String>,
) -> Result<MatrixMessageSearchResult, MatrixAuthCommandError> {
    let payload = core
        .command(CommandEnvelope {
            command: "matrix_message_search".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "term": term,
                "nextToken": next_token,
                "rooms": rooms,
                "senders": senders,
                "order": order,
            }),
        })
        .await
        .map(|response| response.payload)
        .map_err(map_message_search_core_error)?;
    serde_json::from_value(payload).map_err(|_| message_search_response_error())
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

fn message_search_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native message search is unavailable.",
        "v-search.sdk-failed",
    )
}
