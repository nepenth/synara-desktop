//! Desktop bridges for space snapshots and writes through `Core::command`.

use synara_core::app::spaces::{
    NativeRestrictedJoinReparentResult, NativeSpaceChildMutationResult,
    NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot, NativeSpaceParentsSnapshot,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn space_parents_snapshot(
    core: &Core,
) -> Result<NativeSpaceParentsSnapshot, MatrixAuthCommandError> {
    let payload = core
        .space_parents_snapshot()
        .await
        .map_err(map_space_core_error)?;
    Ok(payload)
}

pub(crate) async fn space_hierarchy_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeSpaceHierarchySnapshot, MatrixAuthCommandError> {
    let payload = core
        .space_hierarchy_snapshot(synara_core::core_api::MatrixSpaceHierarchySnapshotRequest {
            room_id,
        })
        .await
        .map_err(map_space_core_error)?;
    Ok(payload)
}

pub(crate) async fn space_children_snapshot(
    core: &Core,
) -> Result<NativeSpaceChildrenSnapshot, MatrixAuthCommandError> {
    let payload = core
        .space_children_snapshot()
        .await
        .map_err(map_space_core_error)?;
    Ok(payload)
}

pub(crate) async fn space_child_set(
    core: &Core,
    parent_id: String,
    child_id: String,
    via: Vec<String>,
    order: Option<String>,
    suggested: Option<bool>,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    let payload = core
        .space_child_set(synara_core::core_api::MatrixSpaceChildSetRequest {
            parent_id,
            child_id,
            via,
            order,
            suggested,
        })
        .await
        .map_err(map_space_core_error)?;
    Ok(payload)
}

pub(crate) async fn space_child_remove(
    core: &Core,
    parent_id: String,
    child_id: String,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    let payload = core
        .space_child_remove(synara_core::core_api::MatrixSpaceChildRemoveRequest {
            parent_id,
            child_id,
        })
        .await
        .map_err(map_space_core_error)?;
    Ok(payload)
}

pub(crate) async fn restricted_join_reparent(
    core: &Core,
    room_id: String,
    remove_parent_id: Option<String>,
    add_parent_id: String,
) -> Result<NativeRestrictedJoinReparentResult, MatrixAuthCommandError> {
    let payload = core
        .restricted_join_reparent(synara_core::core_api::MatrixRestrictedJoinReparentRequest {
            room_id,
            remove_parent_id,
            add_parent_id,
        })
        .await
        .map_err(map_space_core_error)?;
    Ok(payload)
}

fn map_space_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-rooms.2c-space-child-set-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let (code, message) = match diagnostic {
                "v-rooms.2c-room-missing" | "v-rooms.2c-room-not-joined" => (
                    "NotFound",
                    "The native Matrix space child room was not found.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix space child request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let message = match diagnostic {
                "v-rooms.2a-space-child-state-failed" | "v-rooms.2a-space-parents-read-failed" => {
                    "The native Matrix space parent map is unavailable."
                }
                "v-rooms.2b-space-hierarchy-read-failed"
                | "v-rooms.2b-space-hierarchy-page-limit" => {
                    "The native Matrix space hierarchy is unavailable."
                }
                "v-rooms.2c-space-children-read-failed" => {
                    "The native Matrix space child graph is unavailable."
                }
                _ => "The native Matrix space child mutation could not be completed.",
            };
            MatrixAuthCommandError::new("Unknown", message, diagnostic)
        }
    }
}
