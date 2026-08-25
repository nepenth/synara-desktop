//! Shared policy for time-bounded agent approval actions.
//!
//! Presentation belongs to each client. Classification, expiry, allowed
//! background actions, and terminal-decision semantics live here so an OS
//! notification cannot bypass the in-app approval contract.

use serde::{Deserialize, Serialize};

pub const AGENT_APPROVAL_TTL_MS: u64 = 5 * 60 * 1000;
pub const AGENT_APPROVAL_MAX_BODY_CHARS: usize = 100_000;
pub const AGENT_APPROVAL_ACTION_APPROVE_ONCE: &str = "agent-approval.approve-once";
pub const AGENT_APPROVAL_ACTION_DENY: &str = "agent-approval.deny";
pub const AGENT_APPROVAL_REACTION_APPROVE_ONCE: &str = "✅";
pub const AGENT_APPROVAL_REACTION_APPROVE_ALWAYS: &str = "♾️";
pub const AGENT_APPROVAL_REACTION_APPROVE_ALWAYS_TEXT: &str = "♾";
pub const AGENT_APPROVAL_REACTION_DENY: &str = "❌";
pub const AGENT_APPROVAL_REACTION_DENY_ALTERNATE: &str = "❎";
pub const AGENT_APPROVAL_TERMINAL_REACTIONS: [&str; 5] = [
    AGENT_APPROVAL_REACTION_APPROVE_ONCE,
    AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
    AGENT_APPROVAL_REACTION_APPROVE_ALWAYS_TEXT,
    AGENT_APPROVAL_REACTION_DENY,
    AGENT_APPROVAL_REACTION_DENY_ALTERNATE,
];

const APPROVAL_HEADINGS: [&str; 2] = [
    "approval required: dangerous command",
    "dangerous command requires approval",
];

pub fn is_agent_approval_prompt(body: &str) -> bool {
    if body.chars().count() > AGENT_APPROVAL_MAX_BODY_CHARS {
        return false;
    }
    let Some(first_line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return false;
    };
    let normalized = first_line
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let normalized = normalized
        .trim_start_matches(['⚠', '️', ' ', '*'])
        .trim_end_matches([' ', '*', ':'])
        .trim();
    APPROVAL_HEADINGS.contains(&normalized)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentApprovalDecisionStatus {
    Applied,
    AlreadyDecided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentApprovalPlan<'a> {
    pub status: AgentApprovalDecisionStatus,
    pub reaction: Option<&'a str>,
}

/// Validate a background approval action against authoritative event state.
///
/// `existing_reactions` identifies whether each aggregate belongs to the
/// current account. Hermes seeds all three choices as bot-owned reactions, so
/// counts from other senders are not decisions. Once this account has decided on any
/// client, another notification action must not add a contradictory decision.
/// Approve-always is intentionally not accepted from an OS notification; it
/// requires the in-app confirmation route.
pub fn plan_agent_approval<'a, 'b>(
    action_id: &'a str,
    body: &str,
    origin_server_ts: u64,
    now_ms: u64,
    existing_reactions: impl IntoIterator<Item = (&'b str, bool)>,
) -> Result<AgentApprovalPlan<'a>, &'static str> {
    let reaction = match action_id {
        AGENT_APPROVAL_ACTION_APPROVE_ONCE => AGENT_APPROVAL_REACTION_APPROVE_ONCE,
        AGENT_APPROVAL_ACTION_DENY => AGENT_APPROVAL_REACTION_DENY,
        _ => return Err("agent-approval-action-unsupported"),
    };
    if !is_agent_approval_prompt(body) {
        return Err("agent-approval-prompt-invalid");
    }
    if origin_server_ts == 0 || origin_server_ts > now_ms.saturating_add(60_000) {
        return Err("agent-approval-timestamp-invalid");
    }
    if now_ms.saturating_sub(origin_server_ts) >= AGENT_APPROVAL_TTL_MS {
        return Err("agent-approval-expired");
    }
    if existing_reactions
        .into_iter()
        .any(|(key, is_current_account)| {
            is_current_account && AGENT_APPROVAL_TERMINAL_REACTIONS.contains(&key)
        })
    {
        return Ok(AgentApprovalPlan {
            status: AgentApprovalDecisionStatus::AlreadyDecided,
            reaction: None,
        });
    }
    Ok(AgentApprovalPlan {
        status: AgentApprovalDecisionStatus::Applied,
        reaction: Some(reaction),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "Approval Required: Dangerous Command\nCode\nrm file\nReason: test";
    const HERMES_MATRIX_PROMPT: &str = "⚠️ **Dangerous command requires approval**\n```\nrm -rf /tmp/test\n```\nReason: dangerous\n\nReply `!approve` to execute, `!approve session` to approve this pattern for the session, `!approve always` to approve permanently, or `!deny` to cancel.\n\nYou can also react to this prompt:\n✅ = approve once\n♾️ = approve always\n❌ = deny";

    #[test]
    fn classifier_accepts_both_contract_headings() {
        assert!(is_agent_approval_prompt(PROMPT));
        assert!(is_agent_approval_prompt(HERMES_MATRIX_PROMPT));
        assert!(is_agent_approval_prompt(
            "⚠️ **Dangerous command requires approval:**\n```\ntrue\n```"
        ));
        assert!(!is_agent_approval_prompt(
            "A quoted dangerous command requires approval later in this message"
        ));
        assert!(!is_agent_approval_prompt(
            "Dangerous command\nrequires approval"
        ));
        assert!(!is_agent_approval_prompt(
            "Please approve this ordinary request"
        ));
    }

    #[test]
    fn planner_allows_only_bounded_once_or_deny_actions() {
        let plan =
            plan_agent_approval(AGENT_APPROVAL_ACTION_APPROVE_ONCE, PROMPT, 1_000, 1_001, [])
                .unwrap();
        assert_eq!(plan.status, AgentApprovalDecisionStatus::Applied);
        assert_eq!(plan.reaction, Some(AGENT_APPROVAL_REACTION_APPROVE_ONCE));
        assert_eq!(
            plan_agent_approval("agent-approval.approve-always", PROMPT, 1_000, 1_001, []),
            Err("agent-approval-action-unsupported")
        );
    }

    #[test]
    fn planner_rejects_expired_or_future_events() {
        assert_eq!(
            plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                PROMPT,
                1_000,
                1_000 + AGENT_APPROVAL_TTL_MS,
                [],
            ),
            Err("agent-approval-expired")
        );
        assert_eq!(
            plan_agent_approval(AGENT_APPROVAL_ACTION_DENY, PROMPT, 70_001, 10_000, []),
            Err("agent-approval-timestamp-invalid")
        );
    }

    #[test]
    fn any_current_account_terminal_reaction_makes_the_event_decided() {
        for existing in AGENT_APPROVAL_TERMINAL_REACTIONS {
            let plan = plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                PROMPT,
                1_000,
                1_001,
                [(existing, true)],
            )
            .unwrap();
            assert_eq!(plan.status, AgentApprovalDecisionStatus::AlreadyDecided);
            assert_eq!(plan.reaction, None);
        }
    }

    #[test]
    fn hermes_seed_reactions_are_not_user_decisions() {
        let plan = plan_agent_approval(
            AGENT_APPROVAL_ACTION_APPROVE_ONCE,
            PROMPT,
            1_000,
            1_001,
            AGENT_APPROVAL_TERMINAL_REACTIONS.map(|key| (key, false)),
        )
        .unwrap();
        assert_eq!(plan.status, AgentApprovalDecisionStatus::Applied);
    }

    #[test]
    fn hermes_terminal_aliases_are_current_account_decisions() {
        for key in [
            AGENT_APPROVAL_REACTION_APPROVE_ALWAYS_TEXT,
            AGENT_APPROVAL_REACTION_DENY_ALTERNATE,
        ] {
            let plan = plan_agent_approval(
                AGENT_APPROVAL_ACTION_APPROVE_ONCE,
                PROMPT,
                1_000,
                1_001,
                [(key, true)],
            )
            .unwrap();
            assert_eq!(plan.status, AgentApprovalDecisionStatus::AlreadyDecided);
        }
    }
}
