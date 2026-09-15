//! Desktop bridges for `in.synara.agent_approval_history` through `Core::command`.

use synara_core::app::account_data::NativeAgentApprovalHistorySnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn agent_approval_history_snapshot(
    core: &Core,
) -> Result<NativeAgentApprovalHistorySnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_agent_approval_history_snapshot".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_history_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| history_response_error())
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

fn history_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix approval history is unavailable.",
        "agent-approval-history-load-failed",
    )
}
