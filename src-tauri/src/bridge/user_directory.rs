//! Desktop bridge for user-directory search through `Core::command`.

use synara_core::app::user_profile::MatrixUserDirectorySearchResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn user_directory_search(
    core: &Core,
    term: String,
    limit: Option<u64>,
) -> Result<MatrixUserDirectorySearchResult, MatrixAuthCommandError> {
    core.user_directory_search(synara_core::core_api::MatrixUserDirectorySearchRequest {
        term,
        limit,
    })
    .await
    .map_err(map_user_directory_core_error)
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
