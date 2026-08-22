//! Desktop bridge for user-directory search through `Core::command`.

use synara_core::app::user_profile::MatrixUserDirectorySearchResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn user_directory_search(
    core: &Core,
    term: String,
    limit: Option<u64>,
) -> Result<MatrixUserDirectorySearchResult, MatrixAuthCommandError> {
    let payload = core
        .command(CommandEnvelope {
            command: "matrix_user_directory_search".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "term": term,
                "limit": limit,
            }),
        })
        .await
        .map(|response| response.payload)
        .map_err(map_user_directory_core_error)?;
    serde_json::from_value(payload).map_err(|_| user_directory_response_error())
}

fn map_user_directory_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-search.directory-sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "p2-user-directory-search-no-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native user-directory search request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native user-directory search is unavailable.",
            diagnostic,
        ),
    }
}

fn user_directory_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native user-directory search is unavailable.",
        "v-search.directory-sdk-failed",
    )
}
