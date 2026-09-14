//! Desktop bridges for directory search/cancel through `Core::command`.

use synara_core::app::room_directory::{
    DirectoryRoomHitDto, DirectoryRoomTypeFilter, NativeRoomDirectoryPage,
    NativeRoomDirectorySearchResponse,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

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
    let payload = dispatch(
        core,
        "matrix_room_directory_search",
        serde_json::json!({
            "sessionGeneration": session_generation,
            "requestId": request_id,
            "serverName": server_name,
            "term": term,
            "roomType": room_type,
            "thirdPartyInstanceId": third_party_instance_id,
            "limit": limit,
            "since": since,
        }),
    )
    .await?;
    parse_search_response(payload)
}

pub(crate) async fn room_directory_cancel(
    core: &Core,
    session_generation: u64,
    request_id: u64,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_room_directory_cancel",
        serde_json::json!({
            "sessionGeneration": session_generation,
            "requestId": request_id,
        }),
    )
    .await?;
    parse_search_response(payload)
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map(|response| response.payload)
    .map_err(map_directory_search_core_error)
}

fn parse_search_response(
    payload: serde_json::Value,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WireHit {
        room_id: String,
        name: Option<String>,
        topic: Option<String>,
        canonical_alias: Option<String>,
        avatar_url: Option<String>,
        member_count: u32,
        world_readable: bool,
        guest_can_join: bool,
        room_type: String,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WirePage {
        session_generation: u64,
        request_id: u64,
        chunk: Vec<WireHit>,
        prev_batch: Option<String>,
        next_batch: Option<String>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        session_generation: u64,
        request_id: u64,
        status: String,
        page: Option<WirePage>,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| search_response_error())?;
    let status = match wire.status.as_str() {
        "ready" => "ready",
        "stale" => "stale",
        "cancelled" => "cancelled",
        _ => return Err(search_response_error()),
    };
    let page = match wire.page {
        None => None,
        Some(page) => {
            let mut chunk = Vec::with_capacity(page.chunk.len());
            for hit in page.chunk {
                let room_type = match hit.room_type.as_str() {
                    "room" => "room",
                    "space" => "space",
                    _ => return Err(search_response_error()),
                };
                chunk.push(DirectoryRoomHitDto {
                    room_id: hit.room_id,
                    name: hit.name,
                    topic: hit.topic,
                    canonical_alias: hit.canonical_alias,
                    avatar_url: hit.avatar_url,
                    member_count: hit.member_count,
                    world_readable: hit.world_readable,
                    guest_can_join: hit.guest_can_join,
                    room_type,
                });
            }
            Some(NativeRoomDirectoryPage {
                session_generation: page.session_generation,
                request_id: page.request_id,
                chunk,
                prev_batch: page.prev_batch,
                next_batch: page.next_batch,
            })
        }
    };
    Ok(NativeRoomDirectorySearchResponse {
        session_generation: wire.session_generation,
        request_id: wire.request_id,
        status,
        page,
    })
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
            ),
        },
    }
}

fn search_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The public room directory could not be loaded.",
        "v-rooms.directory-sdk-failed",
    )
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
