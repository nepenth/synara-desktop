use super::*;

#[tauri::command]
pub async fn matrix_space_parents_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeSpaceParentsSnapshot, MatrixAuthCommandError> {
    crate::bridge::spaces::space_parents_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_space_hierarchy_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeSpaceHierarchySnapshot, MatrixAuthCommandError> {
    crate::bridge::spaces::space_hierarchy_snapshot(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_space_children_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeSpaceChildrenSnapshot, MatrixAuthCommandError> {
    crate::bridge::spaces::space_children_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_space_child_set(
    core: State<'_, Arc<synara_core::Core>>,
    parent_id: String,
    child_id: String,
    via: Vec<String>,
    order: Option<String>,
    suggested: Option<bool>,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    crate::bridge::spaces::space_child_set(
        core.inner().as_ref(),
        parent_id,
        child_id,
        via,
        order,
        suggested,
    )
    .await
}

#[tauri::command]
pub async fn matrix_space_child_remove(
    core: State<'_, Arc<synara_core::Core>>,
    parent_id: String,
    child_id: String,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    crate::bridge::spaces::space_child_remove(core.inner().as_ref(), parent_id, child_id).await
}

#[tauri::command]
pub async fn matrix_restricted_join_reparent(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    remove_parent_id: Option<String>,
    add_parent_id: String,
) -> Result<NativeRestrictedJoinReparentResult, MatrixAuthCommandError> {
    crate::bridge::spaces::restricted_join_reparent(
        core.inner().as_ref(),
        room_id,
        remove_parent_id,
        add_parent_id,
    )
    .await
}
