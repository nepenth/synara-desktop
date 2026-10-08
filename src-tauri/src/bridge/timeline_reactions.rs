//! Desktop bridges for timeline reaction mutations through `Core::command`.

use synara_core::app::timeline::{NativeAgentApprovalDecisionResult, NativeReactionMutationResult};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn reaction_toggle(
    core: &Core,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    core.timeline_reaction_toggle(synara_core::core_api::MatrixTimelineReactionKeyRequest {
        room_id,
        event_id,
        key,
    })
    .await
    .map_err(map_reaction_core_error)
}

pub(crate) async fn reaction_ensure(
    core: &Core,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    core.reaction_ensure(synara_core::core_api::MatrixTimelineReactionKeyRequest {
        room_id,
        event_id,
        key,
    })
    .await
    .map_err(map_reaction_core_error)
}

pub(crate) async fn agent_approvals_list(
    core: &Core,
    discovery_active: bool,
) -> Result<synara_core::app::timeline::NativeAgentApprovalInboxSnapshot, MatrixAuthCommandError> {
    let response = core
        .agent_approvals_list(synara_core::core_api::MatrixAgentApprovalsListRequest {
            discovery_active,
        })
        .await
        .map_err(map_approval_list_core_error)?;
    Ok(response)
}

fn map_approval_list_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    if error.category == MatrixIpcErrorCategory::Forbidden {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "agent-approval-inbox-no-session",
        )
    } else {
        approval_list_response_error()
    }
}

fn approval_list_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "ApprovalInboxUnavailable",
        "Approval requests could not be loaded. Try again.",
        "agent-approval-inbox-unavailable",
    )
}

pub(crate) async fn agent_approval_decide(
    core: &Core,
    room_id: String,
    event_id: String,
    action_id: String,
) -> Result<NativeAgentApprovalDecisionResult, MatrixAuthCommandError> {
    let response = core
        .agent_approval_decide(synara_core::core_api::MatrixAgentApprovalDecisionRequest {
            room_id,
            event_id,
            action_id,
        })
        .await
        .map_err(map_reaction_core_error)?;
    Ok(response)
}

pub(crate) async fn reaction_redact(
    core: &Core,
    room_id: String,
    target_event_id: String,
    reaction_event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    let response = core
        .reaction_redact(synara_core::core_api::MatrixReactionRedactRequest {
            room_id,
            target_event_id,
            reaction_event_id,
            key,
        })
        .await
        .map_err(map_reaction_core_error)?;
    Ok(response)
}

fn map_reaction_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix reaction operation could not be completed.",
            "v-send.2-reaction-invalid-key",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix reaction operation could not be completed.",
            "v-send.2-reaction-toggle-failed",
        ),
    }
}
