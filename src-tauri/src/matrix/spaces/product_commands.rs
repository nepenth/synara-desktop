use super::*;

#[tauri::command]
pub async fn matrix_space_parents_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeSpaceParentsSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_space_parents(&active.client, active.sync.session_generation())
        .await
        .map_err(map_space_parents_error)
}

#[tauri::command]
pub async fn matrix_space_hierarchy_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeSpaceHierarchySnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_space_hierarchy(&active.client, active.sync.session_generation(), &room_id)
        .await
        .map_err(map_space_hierarchy_error)
}

#[tauri::command]
pub async fn matrix_space_children_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeSpaceChildrenSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_space_children(&active.client, active.sync.session_generation())
        .await
        .map_err(map_space_children_error)
}

#[tauri::command]
pub async fn matrix_space_child_set(
    state: State<'_, MatrixAuthState>,
    parent_id: String,
    child_id: String,
    via: Vec<String>,
    order: Option<String>,
    suggested: Option<bool>,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    set_space_child(
        &active.client,
        &parent_id,
        &child_id,
        &via,
        order.as_deref(),
        suggested,
    )
    .await
    .map_err(map_space_child_mutation_error)
}

#[tauri::command]
pub async fn matrix_space_child_remove(
    state: State<'_, MatrixAuthState>,
    parent_id: String,
    child_id: String,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    remove_space_child(&active.client, &parent_id, &child_id)
        .await
        .map_err(map_space_child_mutation_error)
}

#[tauri::command]
pub async fn matrix_restricted_join_reparent(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    remove_parent_id: Option<String>,
    add_parent_id: String,
) -> Result<NativeRestrictedJoinReparentResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    reparent_restricted_join_allow(
        &active.client,
        &room_id,
        remove_parent_id.as_deref(),
        &add_parent_id,
    )
    .await
    .map_err(map_space_child_mutation_error)
}

pub(super) fn map_space_parents_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix space parent map is unavailable.",
        diagnostic_id,
    )
}

pub(super) fn map_space_hierarchy_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix space hierarchy is unavailable.",
        diagnostic_id,
    )
}

pub(super) fn map_space_children_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix space child graph is unavailable.",
        diagnostic_id,
    )
}

pub(super) fn map_space_child_mutation_error(
    diagnostic_id: &'static str,
) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.2c-invalid-parent"
        | "v-rooms.2c-invalid-child"
        | "v-rooms.2c-invalid-room"
        | "v-rooms.2c-invalid-via"
        | "v-rooms.2c-invalid-order" => (
            "InvalidRequest",
            "The native Matrix space child request is invalid.",
        ),
        "v-rooms.2c-room-missing" | "v-rooms.2c-room-not-joined" => (
            "NotFound",
            "The native Matrix space child room was not found.",
        ),
        _ => (
            "Unknown",
            "The native Matrix space child mutation could not be completed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
