//! Desktop bridges for `in.synara.agent_approval_history` through `Core::command`.

use synara_core::app::account_data::NativeAgentApprovalHistorySnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn agent_approval_history_snapshot(
    core: &Core,
) -> Result<NativeAgentApprovalHistorySnapshot, MatrixAuthCommandError> {
    let response = core
        .agent_approval_history_snapshot()
        .await
        .map_err(map_history_core_error)?;
    Ok(response)
}

fn map_history_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "agent-approval-history-no-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix approval history is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("agent-approval-history-load-failed"),
        ),
    }
}
