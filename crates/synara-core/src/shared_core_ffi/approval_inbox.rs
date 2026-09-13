//! Apple-shell access to the same session-owned approvals index as desktop.
//! No platform scanner or eligibility policy exists on this boundary.
use super::SharedCore;
use crate::app::timeline::{
    NativeAgentApprovalInboxCoverage, NativeAgentApprovalInboxSnapshot,
    NativeAgentApprovalInboxStatus,
};
use crate::transport::{CommandEnvelope, MatrixIpcErrorCategory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentApprovalInboxItemDto {
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub body: String,
    pub origin_server_ts: u64,
    pub expires_at: u64,
    pub status: String,
    pub can_send_reaction: bool,
    pub body_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentApprovalInboxDto {
    pub session_generation: u64,
    pub items: Vec<AgentApprovalInboxItemDto>,
    pub loading: bool,
    pub incomplete: bool,
    pub coverage_window_ms: u64,
    pub coverage: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentApprovalInboxError {
    Failed { code: String, description: String },
}
impl std::fmt::Display for AgentApprovalInboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}
impl std::error::Error for AgentApprovalInboxError {}

fn unavailable(no_session: bool) -> AgentApprovalInboxError {
    let (code, description) = if no_session {
        (
            "agent-approval-inbox-no-session",
            "No native Matrix session is active.",
        )
    } else {
        (
            "agent-approval-inbox-unavailable",
            "Approval requests could not be loaded. Try again.",
        )
    };
    AgentApprovalInboxError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

impl SharedCore {
    /// Pass true only while the approvals page is visible; Core automatically
    /// expires the discovery lease if the shell stops renewing it.
    pub async fn agent_approvals_list(
        &self,
        discovery_active: bool,
    ) -> Result<AgentApprovalInboxDto, AgentApprovalInboxError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: "matrix_agent_approvals_list".to_owned(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({ "discoveryActive": discovery_active }),
            })
            .await
            .map_err(|error| unavailable(error.category == MatrixIpcErrorCategory::Forbidden))?;
        let snapshot: NativeAgentApprovalInboxSnapshot =
            serde_json::from_value(response.payload).map_err(|_| unavailable(false))?;
        Ok(AgentApprovalInboxDto {
            session_generation: snapshot.session_generation,
            loading: snapshot.loading,
            incomplete: snapshot.incomplete,
            coverage_window_ms: snapshot.coverage_window_ms,
            coverage: match snapshot.coverage {
                NativeAgentApprovalInboxCoverage::LatestEvent => "latest_event",
                NativeAgentApprovalInboxCoverage::Discovery => "discovery",
            }
            .to_owned(),
            items: snapshot
                .items
                .into_iter()
                .map(|item| AgentApprovalInboxItemDto {
                    room_id: item.room_id,
                    event_id: item.event_id,
                    sender: item.sender,
                    body: item.body,
                    origin_server_ts: item.origin_server_ts,
                    expires_at: item.expires_at,
                    status: match item.status {
                        NativeAgentApprovalInboxStatus::Pending => "pending",
                        NativeAgentApprovalInboxStatus::Decided => "decided",
                        NativeAgentApprovalInboxStatus::Expired => "expired",
                    }
                    .to_owned(),
                    can_send_reaction: item.can_send_reaction,
                    body_truncated: item.body_truncated,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ffi_approval_inbox_uses_core_lease_and_preserves_authority() {
        use crate::core::CoreState;
        use crate::transport::{CommandFuture, CommandRegistry};
        use std::sync::Arc;
        let mut registry = CommandRegistry::new();
        registry.register("matrix_agent_approvals_list", |_state: Arc<CoreState>, request: CommandEnvelope| -> CommandFuture {
            Box::pin(async move {
                assert_eq!(request.payload, serde_json::json!({ "discoveryActive": true }));
                Ok(serde_json::json!({"sessionGeneration": 12, "loading": false, "incomplete": true, "coverageWindowMs": 300000, "coverage": "discovery",
                    "items": [{"roomId":"!room:example.org","eventId":"$prompt","sender":"@hermes:example.org","body":"preview","originServerTs": 1000,"expiresAt":301000,"status":"pending","canSendReaction":false,"bodyTruncated":true}]}))
            })
        }).unwrap();
        let mut shared = SharedCore::new();
        shared.core = crate::Core::with_registry(
            Arc::new(super::super::IosFailClosedPlatform::new()),
            registry,
        );
        let snapshot = shared.agent_approvals_list(true).await.unwrap();
        assert_eq!(snapshot.session_generation, 12);
        assert_eq!(snapshot.coverage_window_ms, 300000);
        assert_eq!(snapshot.coverage, "discovery");
        assert!(snapshot.incomplete);
        assert!(snapshot.items[0].body_truncated);
        assert!(!snapshot.items[0].can_send_reaction);
        assert_eq!(snapshot.items[0].status, "pending");
    }
    #[tokio::test]
    async fn ffi_approval_inbox_has_a_dedicated_session_error() {
        assert_eq!(
            SharedCore::new()
                .agent_approvals_list(false)
                .await
                .unwrap_err(),
            unavailable(true)
        );
    }
}
