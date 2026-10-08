//! Apple-shell access to the same Core approval-history snapshot as desktop.
//! Summaries only. No command body, tokens, or secret material.
use super::SharedCore;
use crate::app::account_data::{
    NativeAgentApprovalHistorySnapshot, SynaraAgentApprovalHistoryDecision,
    SynaraAgentApprovalHistoryItem,
};
use crate::transport::{MatrixIpcError, MatrixIpcErrorCategory};

const HISTORY_NO_SESSION_CODE: &str = "agent-approval-history-no-session";
const HISTORY_NO_SESSION_DESCRIPTION: &str = "No native Matrix session is active.";
const HISTORY_FAILED_CODE: &str = "agent-approval-history-load-failed";
const HISTORY_FAILED_DESCRIPTION: &str = "The native Matrix approval history is unavailable.";

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AgentApprovalHistoryItemDto {
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub decision: String,
    pub decided_at: f64,
    pub origin_server_ts: f64,
    pub expires_at: f64,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AgentApprovalHistorySnapshotDto {
    pub items: Vec<AgentApprovalHistoryItemDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum AgentApprovalHistoryCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for AgentApprovalHistoryCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}
impl std::error::Error for AgentApprovalHistoryCommandError {}

fn history_failed(code: &str, description: &'static str) -> AgentApprovalHistoryCommandError {
    AgentApprovalHistoryCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_history_core_error(error: MatrixIpcError) -> AgentApprovalHistoryCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            history_failed(HISTORY_NO_SESSION_CODE, HISTORY_NO_SESSION_DESCRIPTION)
        }
        _ => history_failed(
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or(HISTORY_FAILED_CODE),
            HISTORY_FAILED_DESCRIPTION,
        ),
    }
}

fn decision_as_str(decision: SynaraAgentApprovalHistoryDecision) -> String {
    match decision {
        SynaraAgentApprovalHistoryDecision::ApproveOnce => "approve_once",
        SynaraAgentApprovalHistoryDecision::ApproveAlways => "approve_always",
        SynaraAgentApprovalHistoryDecision::Deny => "deny",
    }
    .to_owned()
}

fn item_dto(
    item: SynaraAgentApprovalHistoryItem,
) -> Result<AgentApprovalHistoryItemDto, AgentApprovalHistoryCommandError> {
    if !item.decided_at.is_finite()
        || !item.origin_server_ts.is_finite()
        || !item.expires_at.is_finite()
    {
        return Err(history_failed(
            HISTORY_FAILED_CODE,
            HISTORY_FAILED_DESCRIPTION,
        ));
    }
    Ok(AgentApprovalHistoryItemDto {
        room_id: item.room_id,
        event_id: item.event_id,
        sender: item.sender,
        decision: decision_as_str(item.decision),
        decided_at: item.decided_at,
        origin_server_ts: item.origin_server_ts,
        expires_at: item.expires_at,
        summary: item.summary,
    })
}

fn snapshot_dto(
    payload: NativeAgentApprovalHistorySnapshot,
) -> Result<AgentApprovalHistorySnapshotDto, AgentApprovalHistoryCommandError> {
    let snapshot: NativeAgentApprovalHistorySnapshot = payload;
    Ok(AgentApprovalHistorySnapshotDto {
        items: snapshot
            .items
            .into_iter()
            .map(item_dto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn agent_approval_history_snapshot(
        &self,
    ) -> Result<AgentApprovalHistorySnapshotDto, AgentApprovalHistoryCommandError> {
        let response = self
            .core
            .agent_approval_history_snapshot()
            .await
            .map_err(map_history_core_error)?;
        snapshot_dto(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ffi_approval_history_snapshot_without_session_fails_closed() {
        assert_eq!(
            SharedCore::new()
                .agent_approval_history_snapshot()
                .await
                .unwrap_err(),
            history_failed(HISTORY_NO_SESSION_CODE, HISTORY_NO_SESSION_DESCRIPTION)
        );
    }

    fn core_snapshot() -> NativeAgentApprovalHistorySnapshot {
        serde_json::from_value(serde_json::json!({
            "items": [{
                "roomId": "!room:example.org",
                "eventId": "$prompt",
                "sender": "@hermes:example.org",
                "decision": "approve_always",
                "decidedAt": 9_000.0,
                "originServerTs": 1_000.0,
                "expiresAt": 301_000.0,
                "summary": "rm file"
            }]
        }))
        .unwrap()
    }

    #[test]
    fn ffi_approval_history_snapshot_maps_core_items() {
        let snapshot = snapshot_dto(core_snapshot()).unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].room_id, "!room:example.org");
        assert_eq!(snapshot.items[0].event_id, "$prompt");
        assert_eq!(snapshot.items[0].decision, "approve_always");
        assert_eq!(snapshot.items[0].decided_at, 9_000.0);
        assert_eq!(snapshot.items[0].summary, "rm file");
    }

    #[test]
    fn ffi_approval_history_snapshot_rejects_non_finite_timestamps() {
        let mut snapshot = core_snapshot();
        snapshot.items[0].decided_at = f64::NAN;
        assert_eq!(
            snapshot_dto(snapshot).unwrap_err(),
            history_failed(HISTORY_FAILED_CODE, HISTORY_FAILED_DESCRIPTION)
        );
    }
}
