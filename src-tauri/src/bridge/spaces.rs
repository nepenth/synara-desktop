//! Desktop bridges for space snapshots and writes through `Core::command`.

use synara_core::app::spaces::{
    NativeRestrictedJoinReparentResult, NativeSpaceChildMutationResult,
    NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot, NativeSpaceParentsSnapshot,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn space_parents_snapshot(
    core: &Core,
) -> Result<NativeSpaceParentsSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_space_parents_snapshot",
        serde_json::Value::Null,
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| {
        space_response_error(
            "The native Matrix space parent map is unavailable.",
            "v-rooms.2a-space-parents-read-failed",
        )
    })
}

pub(crate) async fn space_hierarchy_snapshot(
    core: &Core,
    room_id: String,
) -> Result<NativeSpaceHierarchySnapshot, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_space_hierarchy_snapshot",
        serde_json::json!({ "roomId": room_id }),
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| {
        space_response_error(
            "The native Matrix space hierarchy is unavailable.",
            "v-rooms.2b-space-hierarchy-read-failed",
        )
    })
}

pub(crate) async fn space_children_snapshot(
    core: &Core,
) -> Result<NativeSpaceChildrenSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_space_children_snapshot",
        serde_json::Value::Null,
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| {
        space_response_error(
            "The native Matrix space child graph is unavailable.",
            "v-rooms.2c-space-children-read-failed",
        )
    })
}

pub(crate) async fn space_child_set(
    core: &Core,
    parent_id: String,
    child_id: String,
    via: Vec<String>,
    order: Option<String>,
    suggested: Option<bool>,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_space_child_set",
        serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
            "via": via,
            "order": order,
            "suggested": suggested,
        }),
    )
    .await?;
    parse_child_mutation(payload)
}

pub(crate) async fn space_child_remove(
    core: &Core,
    parent_id: String,
    child_id: String,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_space_child_remove",
        serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
        }),
    )
    .await?;
    parse_child_mutation(payload)
}

pub(crate) async fn restricted_join_reparent(
    core: &Core,
    room_id: String,
    remove_parent_id: Option<String>,
    add_parent_id: String,
) -> Result<NativeRestrictedJoinReparentResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_restricted_join_reparent",
        serde_json::json!({
            "roomId": room_id,
            "removeParentId": remove_parent_id,
            "addParentId": add_parent_id,
        }),
    )
    .await?;
    parse_reparent(payload)
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
    .map_err(map_space_core_error)
}

fn parse_child_mutation(
    payload: serde_json::Value,
) -> Result<NativeSpaceChildMutationResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        parent_id: String,
        child_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| mutation_response_error())?;
    let status = match wire.status.as_str() {
        "updated" => "updated",
        "removed" => "removed",
        "skipped" => "skipped",
        _ => return Err(mutation_response_error()),
    };
    Ok(NativeSpaceChildMutationResult {
        parent_id: wire.parent_id,
        child_id: wire.child_id,
        status,
    })
}

fn parse_reparent(
    payload: serde_json::Value,
) -> Result<NativeRestrictedJoinReparentResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| mutation_response_error())?;
    let status = match wire.status.as_str() {
        "updated" => "updated",
        "skipped" => "skipped",
        _ => return Err(mutation_response_error()),
    };
    Ok(NativeRestrictedJoinReparentResult {
        room_id: wire.room_id,
        status,
    })
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

fn space_response_error(message: &'static str, diagnostic: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new("Unknown", message, diagnostic)
}

fn mutation_response_error() -> MatrixAuthCommandError {
    space_response_error(
        "The native Matrix space child mutation could not be completed.",
        "v-rooms.2c-space-child-set-failed",
    )
}
