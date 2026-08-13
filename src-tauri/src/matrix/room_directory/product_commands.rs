//! Product-owned Tauri commands for public-room directory reads.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use super::*;
use crate::matrix::auth::product::{MatrixAuthCommandError, MatrixAuthState};
use crate::matrix::ipc::MAX_WIRE_COUNTER;
use crate::matrix::room_directory::{
    build_public_rooms_request, normalize_search_input, project_response, DirectoryRoomTypeFilter,
    DirectorySearchInput, NativeRoomDirectoryProtocols, NativeRoomDirectorySearchResponse,
};
use tauri::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestAuthority {
    Current,
    Stale,
    Cancelled,
}

#[derive(Default)]
struct DirectoryAuthority {
    current: BTreeMap<u64, u64>,
    cancelled: BTreeSet<(u64, u64)>,
}

static AUTHORITY: OnceLock<Mutex<DirectoryAuthority>> = OnceLock::new();
const MAX_TRACKED_GENERATIONS: usize = 32;

fn authority() -> &'static Mutex<DirectoryAuthority> {
    AUTHORITY.get_or_init(|| Mutex::new(DirectoryAuthority::default()))
}

fn register_request(session_generation: u64, request_id: u64) -> RequestAuthority {
    let mut authority = authority()
        .lock()
        .expect("directory authority is not poisoned");
    if authority
        .cancelled
        .contains(&(session_generation, request_id))
    {
        return RequestAuthority::Cancelled;
    }
    match authority.current.get(&session_generation).copied() {
        Some(current) if request_id <= current => RequestAuthority::Stale,
        _ => {
            authority.current.insert(session_generation, request_id);
            while authority.current.len() > MAX_TRACKED_GENERATIONS {
                let Some(oldest_generation) = authority.current.keys().next().copied() else {
                    break;
                };
                authority.current.remove(&oldest_generation);
            }
            RequestAuthority::Current
        }
    }
}

fn cancel_request(session_generation: u64, request_id: u64) {
    let mut authority = authority()
        .lock()
        .expect("directory authority is not poisoned");
    {
        let current = authority
            .current
            .entry(session_generation)
            .or_insert(request_id);
        if request_id >= *current {
            *current = request_id;
        }
    }
    while authority.current.len() > MAX_TRACKED_GENERATIONS {
        let Some(oldest_generation) = authority.current.keys().next().copied() else {
            break;
        };
        authority.current.remove(&oldest_generation);
    }
    authority.cancelled.insert((session_generation, request_id));
    if authority.cancelled.len() > 256 {
        let keep = authority
            .cancelled
            .iter()
            .rev()
            .take(128)
            .copied()
            .collect::<BTreeSet<_>>();
        authority.cancelled = keep;
    }
}

fn request_authority(session_generation: u64, request_id: u64) -> RequestAuthority {
    let authority = authority()
        .lock()
        .expect("directory authority is not poisoned");
    if authority
        .cancelled
        .contains(&(session_generation, request_id))
    {
        return RequestAuthority::Cancelled;
    }
    match authority.current.get(&session_generation).copied() {
        Some(current) if current == request_id => RequestAuthority::Current,
        _ => RequestAuthority::Stale,
    }
}

fn directory_error(code: &'static str, diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        code,
        "The native Matrix room directory is unavailable.",
        diagnostic_id,
    )
}

fn invalid_directory(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    directory_error("InvalidRequest", diagnostic_id)
}

fn stale_directory(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    directory_error("StaleSessionGeneration", diagnostic_id)
}

fn cancelled_response(
    session_generation: u64,
    request_id: u64,
) -> NativeRoomDirectorySearchResponse {
    NativeRoomDirectorySearchResponse {
        session_generation,
        request_id,
        status: "cancelled",
        page: None,
    }
}

fn stale_response(session_generation: u64, request_id: u64) -> NativeRoomDirectorySearchResponse {
    NativeRoomDirectorySearchResponse {
        session_generation,
        request_id,
        status: "stale",
        page: None,
    }
}

/// Returns only selectable, bounded third-party protocol instances from the
/// managed authenticated client.
#[tauri::command]
pub async fn matrix_room_directory_protocols(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRoomDirectoryProtocols, MatrixAuthCommandError> {
    crate::bridge::directory_protocols::room_directory_protocols(core.inner().as_ref()).await
}

/// Searches the public room directory through the sole managed Matrix SDK
/// client. A newer request or explicit cancellation can only suppress the
/// result; it can never be replaced by a JS implementation.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_room_directory_search(
    state: State<'_, MatrixAuthState>,
    session_generation: u64,
    request_id: u64,
    server_name: Option<String>,
    term: Option<String>,
    room_type: Option<DirectoryRoomTypeFilter>,
    third_party_instance_id: Option<String>,
    limit: u64,
    since: Option<String>,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    if session_generation == 0
        || request_id == 0
        || session_generation > MAX_WIRE_COUNTER
        || request_id > MAX_WIRE_COUNTER
    {
        return Err(invalid_directory("v-rooms.directory-invalid-correlation"));
    }
    let normalized = normalize_search_input(DirectorySearchInput {
        server_name,
        term,
        room_type,
        third_party_instance_id,
        limit,
        since,
    })
    .map_err(invalid_directory)?;

    let (client, current_generation) = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())
            .map_err(|_| directory_error("Forbidden", "v-rooms.directory-requires-session"))?;
        (active.client.clone(), active.sync.session_generation())
    };
    if current_generation != session_generation {
        return Err(stale_directory(
            "v-rooms.directory-stale-generation-before-request",
        ));
    }
    match register_request(session_generation, request_id) {
        RequestAuthority::Current => {}
        RequestAuthority::Stale => return Ok(stale_response(session_generation, request_id)),
        RequestAuthority::Cancelled => {
            return Ok(cancelled_response(session_generation, request_id))
        }
    }
    let request = build_public_rooms_request(&normalized).map_err(invalid_directory)?;
    let response = client
        .public_rooms_filtered(request)
        .await
        .map_err(|_| directory_error("Unknown", "v-rooms.directory-sdk-failed"))?;

    let session = state.session.lock().await;
    let active = require_session(session.as_ref())
        .map_err(|_| directory_error("Forbidden", "v-rooms.directory-requires-session"))?;
    if active.sync.session_generation() != session_generation {
        return Err(stale_directory(
            "v-rooms.directory-stale-generation-after-request",
        ));
    }
    match request_authority(session_generation, request_id) {
        RequestAuthority::Stale => return Ok(stale_response(session_generation, request_id)),
        RequestAuthority::Cancelled => {
            return Ok(cancelled_response(session_generation, request_id))
        }
        RequestAuthority::Current => {}
    }
    let page = project_response(session_generation, request_id, &normalized, response)
        .map_err(|diagnostic_id| directory_error("Unknown", diagnostic_id))?;
    Ok(NativeRoomDirectorySearchResponse {
        session_generation,
        request_id,
        status: "ready",
        page: Some(page),
    })
}

/// Marks a request cancelled. Cancellation is idempotent for the current
/// generation and obsolete requests; a different generation fails closed.
#[tauri::command]
pub async fn matrix_room_directory_cancel(
    state: State<'_, MatrixAuthState>,
    session_generation: u64,
    request_id: u64,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    if session_generation == 0
        || request_id == 0
        || session_generation > MAX_WIRE_COUNTER
        || request_id > MAX_WIRE_COUNTER
    {
        return Err(invalid_directory("v-rooms.directory-invalid-correlation"));
    }
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())
        .map_err(|_| directory_error("Forbidden", "v-rooms.directory-cancel-requires-session"))?;
    if active.sync.session_generation() != session_generation {
        return Err(stale_directory("v-rooms.directory-cancel-stale-generation"));
    }
    cancel_request(session_generation, request_id);
    Ok(cancelled_response(session_generation, request_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_authority_requires_strictly_increasing_ids() {
        let generation = 9_000_000_000;
        assert_eq!(register_request(generation, 1), RequestAuthority::Current);
        assert_eq!(register_request(generation, 1), RequestAuthority::Stale);
        assert_eq!(register_request(generation, 2), RequestAuthority::Current);
        assert_eq!(request_authority(generation, 1), RequestAuthority::Stale);
        assert_eq!(request_authority(generation, 2), RequestAuthority::Current);
    }
}
