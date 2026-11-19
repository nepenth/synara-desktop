//! Core command adapters for user room folders (`in.synara.room_folders`).
//!
//! Folders live on the session's sync owner, which the room list reads, so
//! a folder write shows in the next room-list snapshot before its sync echo.

use super::*;
use crate::app::account_data::{
    NativeRoomFolderCreateResult, NativeRoomFoldersSnapshot, RoomFoldersError, RoomFoldersState,
};

/// Exact React/Tauri envelope payload for `matrix_room_folders_create`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomFoldersCreateRequest {
    pub name: String,
    /// File this room in the new folder in the same write.
    #[serde(default)]
    pub room_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_room_folders_rename`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomFoldersRenameRequest {
    pub folder_id: String,
    pub name: String,
}

/// Exact React/Tauri envelope payload for `matrix_room_folders_delete`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomFoldersDeleteRequest {
    pub folder_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_room_folders_move`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomFoldersMoveRequest {
    pub folder_id: String,
    /// Target position in display order; past the end means last.
    pub to_index: u32,
}

/// Exact React/Tauri envelope payload for `matrix_room_folders_set_room`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomFoldersSetRoomRequest {
    pub room_id: String,
    /// `None` unfiles the room.
    #[serde(default)]
    pub folder_id: Option<String>,
}

pub(super) fn room_folders_owner_error(error: RoomFoldersError) -> MatrixIpcError {
    let category = if error.is_invalid_request() {
        MatrixIpcErrorCategory::SdkInvariant
    } else {
        MatrixIpcErrorCategory::Unknown
    };
    MatrixIpcError::new(category).with_diagnostic(error.diagnostic_id())
}

fn no_session(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden).with_diagnostic(diagnostic_id)
}

/// The sync owner, its client and its folders, or the command's no-session error.
fn folders_owner(
    state: &Arc<CoreState>,
    no_session_id: &'static str,
) -> Result<(Arc<SyncServiceOwner>, matrix_sdk::Client), MatrixIpcError> {
    let owner = state
        .sync_owner()?
        .ok_or_else(|| no_session(no_session_id))?;
    let client = owner.room_list_service().client().clone();
    Ok((owner, client))
}

fn folders(owner: &SyncServiceOwner) -> &RoomFoldersState {
    owner.room_folders()
}

/// Typed `matrix_room_folders_snapshot`.
pub(super) async fn room_folders_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeRoomFoldersSnapshot, MatrixIpcError> {
    let (owner, client) = folders_owner(state, "p2-room-folders-snapshot-no-session")?;
    folders(&owner)
        .snapshot(&client, owner.session_generation())
        .await
        .map_err(room_folders_owner_error)
}

/// Typed `matrix_room_folders_create`.
pub(super) async fn room_folders_create(
    state: &Arc<CoreState>,
    payload: MatrixRoomFoldersCreateRequest,
) -> Result<NativeRoomFolderCreateResult, MatrixIpcError> {
    let (owner, client) = folders_owner(state, "p2-room-folders-create-no-session")?;
    folders(&owner)
        .create(
            &client,
            owner.session_generation(),
            &payload.name,
            payload.room_id.as_deref(),
        )
        .await
        .map_err(room_folders_owner_error)
}

/// Typed `matrix_room_folders_rename`.
pub(super) async fn room_folders_rename(
    state: &Arc<CoreState>,
    payload: MatrixRoomFoldersRenameRequest,
) -> Result<NativeRoomFoldersSnapshot, MatrixIpcError> {
    let (owner, client) = folders_owner(state, "p2-room-folders-rename-no-session")?;
    folders(&owner)
        .rename(
            &client,
            owner.session_generation(),
            &payload.folder_id,
            &payload.name,
        )
        .await
        .map_err(room_folders_owner_error)
}

/// Typed `matrix_room_folders_delete`.
pub(super) async fn room_folders_delete(
    state: &Arc<CoreState>,
    payload: MatrixRoomFoldersDeleteRequest,
) -> Result<NativeRoomFoldersSnapshot, MatrixIpcError> {
    let (owner, client) = folders_owner(state, "p2-room-folders-delete-no-session")?;
    folders(&owner)
        .delete(&client, owner.session_generation(), &payload.folder_id)
        .await
        .map_err(room_folders_owner_error)
}

/// Typed `matrix_room_folders_move`.
pub(super) async fn room_folders_move(
    state: &Arc<CoreState>,
    payload: MatrixRoomFoldersMoveRequest,
) -> Result<NativeRoomFoldersSnapshot, MatrixIpcError> {
    let (owner, client) = folders_owner(state, "p2-room-folders-move-no-session")?;
    let to_index = usize::try_from(payload.to_index).unwrap_or(usize::MAX);
    folders(&owner)
        .move_to(
            &client,
            owner.session_generation(),
            &payload.folder_id,
            to_index,
        )
        .await
        .map_err(room_folders_owner_error)
}

/// Typed `matrix_room_folders_set_room`.
pub(super) async fn room_folders_set_room(
    state: &Arc<CoreState>,
    payload: MatrixRoomFoldersSetRoomRequest,
) -> Result<NativeRoomFoldersSnapshot, MatrixIpcError> {
    let (owner, client) = folders_owner(state, "p2-room-folders-set-room-no-session")?;
    folders(&owner)
        .set_room(
            &client,
            owner.session_generation(),
            &payload.room_id,
            payload.folder_id.as_deref(),
        )
        .await
        .map_err(room_folders_owner_error)
}

#[cfg(test)]
fn json_command<P, R, F, Fut>(
    state: Arc<CoreState>,
    request: CommandEnvelope,
    invalid: &'static str,
    serialization: &'static str,
    run: F,
) -> CommandFuture
where
    P: serde::de::DeserializeOwned + Send + 'static,
    R: Serialize,
    F: FnOnce(Arc<CoreState>, P) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<R, MatrixIpcError>> + Send,
{
    Box::pin(async move {
        let payload: P =
            serde_json::from_value(request.payload).map_err(|_| core_state_error(invalid))?;
        let response = run(state, payload).await?;
        serde_json::to_value(response).map_err(|_| core_state_error(serialization))
    })
}

#[cfg(test)]
fn matrix_room_folders_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-room-folders-snapshot-invalid-payload"));
        }
        let response = room_folders_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-folders-serialization-failed"))
    })
}

#[cfg(test)]
fn matrix_room_folders_create(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    json_command(
        state,
        request,
        "p2-room-folders-create-invalid-payload",
        "p2-room-folders-serialization-failed",
        |state, payload| async move { room_folders_create(&state, payload).await },
    )
}

#[cfg(test)]
fn matrix_room_folders_rename(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    json_command(
        state,
        request,
        "p2-room-folders-rename-invalid-payload",
        "p2-room-folders-serialization-failed",
        |state, payload| async move { room_folders_rename(&state, payload).await },
    )
}

#[cfg(test)]
fn matrix_room_folders_delete(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    json_command(
        state,
        request,
        "p2-room-folders-delete-invalid-payload",
        "p2-room-folders-serialization-failed",
        |state, payload| async move { room_folders_delete(&state, payload).await },
    )
}

#[cfg(test)]
fn matrix_room_folders_move(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    json_command(
        state,
        request,
        "p2-room-folders-move-invalid-payload",
        "p2-room-folders-serialization-failed",
        |state, payload| async move { room_folders_move(&state, payload).await },
    )
}

#[cfg(test)]
fn matrix_room_folders_set_room(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    json_command(
        state,
        request,
        "p2-room-folders-set-room-invalid-payload",
        "p2-room-folders-serialization-failed",
        |state, payload| async move { room_folders_set_room(&state, payload).await },
    )
}

#[cfg(test)]
type JsonAdapter = fn(Arc<CoreState>, CommandEnvelope) -> CommandFuture;

/// Registers this domain's JSON adapters with the test-only command registry.
#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    let commands: [(&'static str, JsonAdapter); 6] = [
        ("matrix_room_folders_create", matrix_room_folders_create),
        ("matrix_room_folders_delete", matrix_room_folders_delete),
        ("matrix_room_folders_move", matrix_room_folders_move),
        ("matrix_room_folders_rename", matrix_room_folders_rename),
        ("matrix_room_folders_set_room", matrix_room_folders_set_room),
        ("matrix_room_folders_snapshot", matrix_room_folders_snapshot),
    ];
    for (name, handler) in commands {
        registry
            .register(name, handler)
            .expect("built-in room-folder commands must remain in the command census");
    }
}
