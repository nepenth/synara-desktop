//! Desktop bridge for `matrix_message_search` through `Core::command`.

use synara_core::app::search::MatrixMessageSearchResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

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
    let mut payload = serde_json::json!({
        "term": term,
        "nextToken": next_token,
        "rooms": rooms,
        "senders": senders,
        "order": order,
    });
    if let Some(kind) = listing_kind {
        payload["listingKind"] = serde_json::Value::String(kind);
    }
    if let Some(from_ts) = from_ts {
        payload["fromTs"] = serde_json::json!(from_ts);
    }
    if let Some(to_ts) = to_ts {
        payload["toTs"] = serde_json::json!(to_ts);
    }
    let payload = core
        .command(CommandEnvelope {
            command: "matrix_message_search".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
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
