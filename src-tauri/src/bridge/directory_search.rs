//! Desktop bridges for directory search/cancel through `Core::command`.

use synara_core::app::room_directory::{
    DirectoryRoomTypeFilter, NativeRoomDirectorySearchResponse,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

#[allow(clippy::too_many_arguments)] // Stable directory-search IPC fields are intentionally explicit.
pub(crate) async fn room_directory_search(
    core: &Core,
    session_generation: u64,
    request_id: u64,
    server_name: Option<String>,
    term: Option<String>,
    room_type: Option<DirectoryRoomTypeFilter>,
    third_party_instance_id: Option<String>,
    limit: u64,
    since: Option<String>,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    let payload = core
        .room_directory_search(synara_core::core_api::MatrixRoomDirectorySearchRequest {
            session_generation,
            request_id,
            server_name,
            term,
            room_type,
            third_party_instance_id,
            limit,
            since,
        })
        .await
        .map_err(map_directory_search_core_error)?;
    Ok(payload)
}

pub(crate) async fn room_directory_cancel(
    core: &Core,
    session_generation: u64,
    request_id: u64,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    let payload = core
        .room_directory_cancel(synara_core::core_api::MatrixRoomDirectoryCancelRequest {
            session_generation,
            request_id,
        })
        .await
        .map_err(map_directory_search_core_error)?;
    Ok(payload)
}

fn map_directory_search_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-rooms.directory-sdk-failed");
    let (code, message) = directory_search_user_error(diagnostic, error.category);
    MatrixAuthCommandError::new(code, message, diagnostic)
}

fn directory_search_user_error(
    diagnostic: &str,
    category: MatrixIpcErrorCategory,
) -> (&'static str, &'static str) {
    match diagnostic {
        "p2-room-directory-search-no-session"
        | "p2-room-directory-cancel-no-session"
        | "v-rooms.directory-requires-session"
        | "v-send.r-room-profile-join-rule-requires-session" => (
            "Forbidden",
            "No native Matrix session is active.",
        ),
        "v-rooms.directory-federation-forbidden" => (
            "Forbidden",
            "This server does not allow public room directory queries over federation. The remote homeserver must enable allow_public_rooms_over_federation.",
        ),
        "v-rooms.directory-server-not-found" => ("NotFound", "That Matrix server was not found."),
        "v-rooms.directory-network-failed" => (
            "Connectivity",
            "Could not reach the room directory. Check your connection and try again.",
        ),
        "v-rooms.directory-invalid-server" => {
            ("InvalidRequest", "That is not a valid Matrix server name.")
        }
        "v-rooms.directory-invalid-limit"
        | "v-rooms.directory-invalid-term"
        | "v-rooms.directory-invalid-instance"
        | "v-rooms.directory-invalid-since"
        | "v-rooms.directory-invalid-correlation" => (
            "InvalidRequest",
            "The room directory request is invalid.",
        ),
        "v-rooms.directory-invalid-hit" | "v-rooms.directory-hit-cap" => (
            "InvalidRequest",
            "The public room directory could not be loaded.",
        ),
        "v-rooms.directory-rate-limited" => (
            "RateLimited",
            "The room directory is rate-limited. Try again in a moment.",
        ),
        "v-rooms.directory-sdk-failed" => (
            "Unknown",
            "The public room directory could not be loaded.",
        ),
        _ => match category {
            MatrixIpcErrorCategory::Forbidden => {
                ("Forbidden", "No native Matrix session is active.")
            }
            MatrixIpcErrorCategory::StaleSessionGeneration => (
                "StaleSessionGeneration",
                "Native Matrix room directory is unavailable.",
            ),
            MatrixIpcErrorCategory::SdkInvariant => (
                "InvalidRequest",
                "The room directory request is invalid.",
            ),
            MatrixIpcErrorCategory::Connectivity => (
                "Connectivity",
                "Could not reach the room directory. Check your connection and try again.",
            ),
            MatrixIpcErrorCategory::RateLimited => (
                "RateLimited",
                "The room directory is rate-limited. Try again in a moment.",
            ),
            MatrixIpcErrorCategory::HomeserverUnavailable => {
                ("NotFound", "That Matrix server was not found.")
            }
            _ => (
                "Unknown",
                "The public room directory could not be loaded.",
            )}}
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_core::transport::MatrixIpcError;

    #[test]
    fn federation_forbidden_is_not_collapsed_to_unavailable() {
        let error = map_directory_search_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("v-rooms.directory-federation-forbidden"),
        );
        assert_eq!(error.code, "Forbidden");
        assert!(error.message.contains("allow_public_rooms_over_federation"));
        assert_eq!(
            error.diagnostic_id,
            "v-rooms.directory-federation-forbidden"
        );
    }

    #[test]
    fn invalid_server_name_has_a_specific_message() {
        let error = map_directory_search_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms.directory-invalid-server"),
        );
        assert_eq!(error.message, "That is not a valid Matrix server name.");
    }

    #[test]
    fn no_session_does_not_use_the_federation_copy() {
        let error = map_directory_search_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-search-no-session"),
        );
        assert_eq!(error.message, "No native Matrix session is active.");
    }

    #[test]
    fn invalid_hit_and_unknown_diagnostics_stay_static() {
        let invalid_hit = map_directory_search_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-rooms.directory-invalid-hit"),
        );
        assert_eq!(invalid_hit.code, "InvalidRequest");
        assert_eq!(
            invalid_hit.message,
            "The public room directory could not be loaded."
        );

        let unknown = map_directory_search_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("server-supplied-M_UNKNOWN"),
        );
        assert_eq!(unknown.code, "Unknown");
        assert_eq!(
            unknown.message,
            "The public room directory could not be loaded."
        );
        assert!(!unknown.message.contains("M_UNKNOWN"));
    }
}
