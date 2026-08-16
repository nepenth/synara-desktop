//! Product-owned Tauri commands for public-room directory reads.

use super::*;
use crate::matrix::room_directory::{
    DirectoryRoomTypeFilter, NativeRoomDirectoryProtocols, NativeRoomDirectorySearchResponse,
};

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
    core: State<'_, Arc<synara_core::Core>>,
    session_generation: u64,
    request_id: u64,
    server_name: Option<String>,
    term: Option<String>,
    room_type: Option<DirectoryRoomTypeFilter>,
    third_party_instance_id: Option<String>,
    limit: u64,
    since: Option<String>,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    crate::bridge::directory_search::room_directory_search(
        core.inner().as_ref(),
        session_generation,
        request_id,
        server_name,
        term,
        room_type,
        third_party_instance_id,
        limit,
        since,
    )
    .await
}

/// Marks a request cancelled. Cancellation is idempotent for the current
/// generation and obsolete requests; a different generation fails closed.
#[tauri::command]
pub async fn matrix_room_directory_cancel(
    core: State<'_, Arc<synara_core::Core>>,
    session_generation: u64,
    request_id: u64,
) -> Result<NativeRoomDirectorySearchResponse, MatrixAuthCommandError> {
    crate::bridge::directory_search::room_directory_cancel(
        core.inner().as_ref(),
        session_generation,
        request_id,
    )
    .await
}
