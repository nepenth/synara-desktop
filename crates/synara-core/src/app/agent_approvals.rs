//! Shared policy for time-bounded agent approval actions.
//!
//! Presentation belongs to each client. Classification, expiry, allowed
//! background actions, and terminal-decision semantics live here so an OS
//! notification cannot bypass the in-app approval contract.

use serde::{Deserialize, Serialize};

pub const AGENT_APPROVAL_TTL_MS: u64 = 5 * 60 * 1000;
pub const AGENT_APPROVAL_MAX_BODY_CHARS: usize = 100_000;
pub const AGENT_APPROVAL_ACTION_APPROVE_ONCE: &str = "agent-approval.approve-once";
pub const AGENT_APPROVAL_ACTION_APPROVE_ALWAYS: &str = "agent-approval.approve-always";
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

/// Classify a prompt only when it came from another Matrix account.
///
/// Hermes does not currently attach a signed, machine-readable approval marker
/// or bot identity to the event. This is therefore the strongest client-side
/// eligibility rule available from the Matrix event itself: exact prompt
/// structure, a remote event resolved by the Core owner, and a sender distinct
/// from the account that will make the decision. Hermes remains the authority
/// that binds a reaction to a live pending command and rejects unauthorized or
/// spoofed events.
pub fn is_eligible_agent_approval_prompt(
    body: &str,
    prompt_sender_id: &str,
    current_user_id: &str,
) -> bool {
    !prompt_sender_id.is_empty()
        && !current_user_id.is_empty()
        && prompt_sender_id != current_user_id
        && is_agent_approval_prompt(body)
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
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

/// Shared classification for inbox presentation and authoritative submission.
/// Expired history can remain visible; the planner rejects it before sending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentApprovalClassification {
    pub expires_at: u64,
    pub expired: bool,
    pub decided: bool,
}

pub fn classify_agent_approval<'a>(
    body: &str,
    prompt_sender_id: &str,
    current_user_id: &str,
    origin_server_ts: u64,
    now_ms: u64,
    existing_reactions: impl IntoIterator<Item = (&'a str, bool)>,
) -> Result<AgentApprovalClassification, &'static str> {
    if !is_eligible_agent_approval_prompt(body, prompt_sender_id, current_user_id) {
        return Err("agent-approval-prompt-invalid");
    }
    if origin_server_ts == 0 || origin_server_ts > now_ms.saturating_add(60_000) {
        return Err("agent-approval-timestamp-invalid");
    }
    Ok(AgentApprovalClassification {
        expires_at: origin_server_ts.saturating_add(AGENT_APPROVAL_TTL_MS),
        expired: now_ms.saturating_sub(origin_server_ts) >= AGENT_APPROVAL_TTL_MS,
        decided: existing_reactions
            .into_iter()
            .any(|(key, own)| own && AGENT_APPROVAL_TERMINAL_REACTIONS.contains(&key)),
    })
}

/// Validate an approval action against authoritative event state.
///
/// `existing_reactions` identifies whether each aggregate belongs to the
/// current account. Hermes seeds all three choices as bot-owned reactions, so
/// counts from other senders are not decisions. Once this account has decided on any
/// client, another notification action must not add a contradictory decision.
/// Platforms must keep approve-always off OS notification surfaces; Core accepts
/// it here only because confirmed in-app decisions use this same owner route.
pub fn plan_agent_approval<'a, 'b>(
    action_id: &'a str,
    body: &str,
    prompt_sender_id: &str,
    current_user_id: &str,
    origin_server_ts: u64,
    now_ms: u64,
    existing_reactions: impl IntoIterator<Item = (&'b str, bool)>,
) -> Result<AgentApprovalPlan<'a>, &'static str> {
    let reaction = match action_id {
        AGENT_APPROVAL_ACTION_APPROVE_ONCE => AGENT_APPROVAL_REACTION_APPROVE_ONCE,
        AGENT_APPROVAL_ACTION_APPROVE_ALWAYS => AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
        AGENT_APPROVAL_ACTION_DENY => AGENT_APPROVAL_REACTION_DENY,
        _ => return Err("agent-approval-action-unsupported"),
    };
    let classification = classify_agent_approval(
        body,
        prompt_sender_id,
        current_user_id,
        origin_server_ts,
        now_ms,
        existing_reactions,
    )?;
    if classification.expired {
        return Err("agent-approval-expired");
    }
    if classification.decided {
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

pub const AGENT_APPROVAL_HISTORY_SUMMARY_MAX_CHARS: usize = 240;

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_disallowed_summary_char(ch: char) -> bool {
    ch.is_control()
        || matches!(
            ch,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
}

fn first_display_line(value: &str) -> &str {
    value
        .split(['\u{000B}', '\u{000C}', '\u{2028}', '\u{2029}'])
        .next()
        .unwrap_or(value)
}

fn is_path_arg(token: &str) -> bool {
    if token.starts_with('/')
        || token.starts_with("~/")
        || token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with("file:")
        || token.starts_with("\\\\")
    {
        return true;
    }
    if token.contains('/') || token.contains('\\') {
        return !token.starts_with('-');
    }
    let bytes = token.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn looks_like_secret_name(name: &str) -> bool {
    let n = name.trim_start_matches(['-', '_']).to_ascii_lowercase();
    n == "auth"
        || n.ends_with("_auth")
        || n.ends_with("-auth")
        || n.ends_with("token")
        || n.ends_with("secret")
        || n.ends_with("password")
        || n.ends_with("passwd")
        || n.ends_with("authorization")
        || n.ends_with("credential")
        || n.ends_with("api_key")
        || n.ends_with("apikey")
        || n.ends_with("api-key")
        || n.ends_with("access_key")
        || n.ends_with("bearer")
}

fn is_secret_flag(token: &str) -> bool {
    looks_like_secret_name(token) && token.starts_with('-') && !token.contains('=')
}

fn is_known_secret_literal(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    lower == "bearer"
        || lower.starts_with("sk-")
        || lower.starts_with("ghp_")
        || lower.starts_with("github_pat_")
        || lower.starts_with("xox")
}

fn redact_assignment(token: &str, redact_paths: bool) -> Option<String> {
    let (name, value) = token.split_once('=')?;
    if name.is_empty() || value.is_empty() {
        return None;
    }
    if looks_like_secret_name(name) {
        return Some(format!("{name}=<redacted>"));
    }
    if redact_paths && is_path_arg(value) {
        return Some(format!("{name}=<path>"));
    }
    None
}

fn redact_agent_approval_history_preview(value: &str) -> String {
    redact_agent_approval_preview(value, true)
}

/// Secret-like values are always redacted. Paths are replaced only for
/// server-readable account data; on-device notifications keep them.
fn redact_agent_approval_preview(value: &str, redact_paths: bool) -> String {
    let tokens: Vec<&str> = value.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut redact_next = false;
    for token in tokens {
        if redact_next {
            out.push(if redact_paths && is_path_arg(token) {
                "<path>".to_owned()
            } else {
                "<redacted>".to_owned()
            });
            redact_next = false;
            continue;
        }
        if let Some(redacted) = redact_assignment(token, redact_paths) {
            out.push(redacted);
            continue;
        }
        if is_secret_flag(token) {
            out.push(token.to_owned());
            redact_next = true;
            continue;
        }
        if is_known_secret_literal(token) {
            out.push("<redacted>".to_owned());
            continue;
        }
        if redact_paths && is_path_arg(token) {
            out.push("<path>".to_owned());
            continue;
        }
        out.push(token.to_owned());
    }
    out.join(" ")
}

/// Single visible preview line: no extra fence lines, bidi/overrides, or
/// control characters. Account data is server-readable plaintext, so path-like
/// args and secret-like assignments are replaced with placeholders.
pub(crate) fn sanitize_agent_approval_history_summary(value: &str) -> String {
    redact_agent_approval_history_preview(&clean_display_line(value))
        .chars()
        .take(AGENT_APPROVAL_HISTORY_SUMMARY_MAX_CHARS)
        .collect()
}

/// First display line with bidi/override/control characters removed and
/// whitespace collapsed.
fn clean_display_line(value: &str) -> String {
    let cleaned: String = first_display_line(value)
        .chars()
        .filter(|ch| !is_disallowed_summary_char(*ch))
        .collect();
    collapse_whitespace(&cleaned)
}

fn preview_from_line(value: &str) -> Option<String> {
    let preview = sanitize_agent_approval_history_summary(value);
    if preview.is_empty() {
        None
    } else {
        Some(preview)
    }
}

fn first_useful_command_line(block: &str) -> Option<String> {
    block.lines().find_map(|line| {
        let display = first_display_line(line.trim());
        if display.is_empty() {
            return None;
        }
        let lowered = display.to_ascii_lowercase();
        if lowered == "code" || lowered == "copy" {
            return None;
        }
        preview_from_line(display)
    })
}

fn fenced_command_block(body: &str) -> Option<&str> {
    let start = body.find("```")?;
    let after_ticks = body.get(start + 3..)?;
    let after_info = after_ticks.split_once('\n')?.1;
    Some(after_info.split("```").next().unwrap_or(after_info))
}

fn extract_fenced_command_preview(body: &str) -> Option<String> {
    first_useful_command_line(fenced_command_block(body)?)
}

fn extract_labeled_command_preview(body: &str) -> Option<String> {
    preview_from_line(first_display_line(extract_labeled_command_line(body)?))
}

fn extract_labeled_command_line(body: &str) -> Option<&str> {
    let mut lines = body.lines().peekable();
    while let Some(line) = lines.next() {
        let lowered = line.trim().to_ascii_lowercase();
        if lowered != "code" && lowered != "copy" {
            continue;
        }
        for candidate in lines.by_ref() {
            let trimmed = candidate.trim();
            if trimmed.is_empty() {
                continue;
            }
            let lowered = trimmed.to_ascii_lowercase();
            if lowered == "code" || lowered == "copy" {
                continue;
            }
            if trimmed.to_ascii_lowercase().starts_with("reason:")
                || trimmed.to_ascii_lowercase().starts_with("reply ")
            {
                break;
            }
            return Some(trimmed);
        }
        break;
    }
    None
}

fn extract_reason_preview(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed
            .strip_prefix("Reason:")
            .or_else(|| trimmed.strip_prefix("reason:"))
        else {
            continue;
        };
        return preview_from_line(first_display_line(rest));
    }
    None
}

fn extract_fallback_preview(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let display = first_display_line(line.trim());
        if display.is_empty() {
            return None;
        }
        let lowered = display.to_ascii_lowercase();
        if APPROVAL_HEADINGS
            .iter()
            .any(|heading| lowered.contains(heading))
            || lowered.starts_with("reply ")
            || lowered == "code"
            || lowered == "copy"
        {
            return None;
        }
        preview_from_line(display)
    })
}

/// Bounded account-data summary: command preview line, else Reason, else a
/// non-heading line. Never the full command body.
pub fn agent_approval_history_summary(body: &str) -> String {
    extract_fenced_command_preview(body)
        .or_else(|| extract_labeled_command_preview(body))
        .or_else(|| extract_reason_preview(body))
        .or_else(|| extract_fallback_preview(body))
        .unwrap_or_default()
}

pub const AGENT_APPROVAL_NOTIFICATION_REASON_MAX_CHARS: usize = 120;
pub const AGENT_APPROVAL_NOTIFICATION_COMMAND_MAX_CHARS: usize = 240;
const AGENT_APPROVAL_NOTIFICATION_LINE_SEPARATOR: &str = " ↵ ";

/// What an OS notification may say about a prompt: the Reason line and the
/// requested command. Both are display-safe and bounded; secret-like values
/// are redacted, but paths stay because the text never leaves the device.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentApprovalNotificationSummary {
    pub reason: Option<String>,
    pub command: Option<String>,
}

fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    out.truncate(out.trim_end().len());
    out.push('…');
    out
}

fn notification_text(value: &str, max_chars: usize) -> Option<String> {
    let redacted = redact_agent_approval_preview(value, false);
    if redacted.is_empty() {
        None
    } else {
        Some(truncate_with_ellipsis(&redacted, max_chars))
    }
}

fn is_env_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Leading `NAME=value` assignments are environment setup, not the command.
fn strip_env_prefix(line: &str) -> &str {
    let mut rest = line;
    loop {
        let Some((token, tail)) = rest.split_once(' ') else {
            return rest;
        };
        if !is_env_assignment(token) {
            return rest;
        }
        rest = tail.trim_start();
    }
}

fn notification_command(body: &str) -> Option<String> {
    let lines: Vec<String> = match fenced_command_block(body) {
        Some(block) => block
            .lines()
            .map(clean_display_line)
            .filter(|line| !line.is_empty())
            .collect(),
        None => extract_labeled_command_line(body)
            .map(|line| vec![clean_display_line(line)])
            .unwrap_or_default(),
    };
    let joined = lines
        .iter()
        .map(|line| strip_env_prefix(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(AGENT_APPROVAL_NOTIFICATION_LINE_SEPARATOR);
    notification_text(&joined, AGENT_APPROVAL_NOTIFICATION_COMMAND_MAX_CHARS)
}

fn notification_reason(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed
            .strip_prefix("Reason:")
            .or_else(|| trimmed.strip_prefix("reason:"))?;
        notification_text(
            &clean_display_line(rest),
            AGENT_APPROVAL_NOTIFICATION_REASON_MAX_CHARS,
        )
    })
}

/// Callers must classify the prompt first; this only extracts display text.
pub fn agent_approval_notification_summary(body: &str) -> AgentApprovalNotificationSummary {
    if body.chars().count() > AGENT_APPROVAL_MAX_BODY_CHARS {
        return AgentApprovalNotificationSummary::default();
    }
    AgentApprovalNotificationSummary {
        reason: notification_reason(body),
        command: notification_command(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "Approval Required: Dangerous Command\nCode\nrm file\nReason: test";
    const HERMES_MATRIX_PROMPT: &str = "⚠️ **Dangerous command requires approval**\n```\nrm -rf /tmp/test\n```\nReason: dangerous\n\nReply `!approve` to execute, `!approve session` to approve this pattern for the session, `!approve always` to approve permanently, or `!deny` to cancel.\n\nYou can also react to this prompt:\n✅ = approve once\n♾️ = approve always\n❌ = deny";
    const HERMES: &str = "@hermes:example.org";
    const CURRENT_USER: &str = "@alice:example.org";

    #[test]
    fn classifier_accepts_both_contract_headings() {
        assert!(is_agent_approval_prompt(PROMPT));
        assert!(is_agent_approval_prompt(HERMES_MATRIX_PROMPT));
        assert!(is_agent_approval_prompt(
            "⚠️ Dangerous command requires approval"
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
        assert!(!is_agent_approval_prompt(
            "> ⚠️ **Dangerous command requires approval**\n> quoted prompt"
        ));
    }

    #[test]
    fn planner_allows_only_contract_reaction_actions() {
        let plan = plan_agent_approval(
            AGENT_APPROVAL_ACTION_APPROVE_ONCE,
            PROMPT,
            HERMES,
            CURRENT_USER,
            1_000,
            1_001,
            [],
        )
        .unwrap();
        assert_eq!(plan.status, AgentApprovalDecisionStatus::Applied);
        assert_eq!(plan.reaction, Some(AGENT_APPROVAL_REACTION_APPROVE_ONCE));
        assert_eq!(
            plan_agent_approval(
                AGENT_APPROVAL_ACTION_APPROVE_ALWAYS,
                PROMPT,
                HERMES,
                CURRENT_USER,
                1_000,
                1_001,
                [],
            )
            .unwrap()
            .reaction,
            Some(AGENT_APPROVAL_REACTION_APPROVE_ALWAYS)
        );
        assert_eq!(
            plan_agent_approval(
                "agent-approval.approve-session",
                PROMPT,
                HERMES,
                CURRENT_USER,
                1_000,
                1_001,
                [],
            ),
            Err("agent-approval-action-unsupported")
        );
    }

    #[test]
    fn planner_rejects_own_account_and_non_prompt_senders() {
        assert_eq!(
            plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                PROMPT,
                CURRENT_USER,
                CURRENT_USER,
                1_000,
                1_001,
                [],
            ),
            Err("agent-approval-prompt-invalid")
        );
        assert_eq!(
            plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                "ordinary message",
                HERMES,
                CURRENT_USER,
                1_000,
                1_001,
                [],
            ),
            Err("agent-approval-prompt-invalid")
        );
    }

    #[test]
    fn planner_rejects_expired_or_future_events() {
        assert_eq!(
            plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                PROMPT,
                HERMES,
                CURRENT_USER,
                1_000,
                1_000 + AGENT_APPROVAL_TTL_MS,
                [],
            ),
            Err("agent-approval-expired")
        );
        assert_eq!(
            plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                PROMPT,
                HERMES,
                CURRENT_USER,
                70_001,
                10_000,
                [],
            ),
            Err("agent-approval-timestamp-invalid")
        );
    }

    #[test]
    fn any_current_account_terminal_reaction_makes_the_event_decided() {
        for existing in AGENT_APPROVAL_TERMINAL_REACTIONS {
            let plan = plan_agent_approval(
                AGENT_APPROVAL_ACTION_DENY,
                PROMPT,
                HERMES,
                CURRENT_USER,
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
            HERMES,
            CURRENT_USER,
            1_000,
            1_001,
            AGENT_APPROVAL_TERMINAL_REACTIONS.map(|key| (key, false)),
        )
        .unwrap();
        assert_eq!(plan.status, AgentApprovalDecisionStatus::Applied);
    }

    #[test]
    fn history_summary_prefers_fenced_command_preview_line() {
        assert_eq!(
            agent_approval_history_summary(HERMES_MATRIX_PROMPT),
            "rm -rf <path>"
        );
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\nCode\nrm file\nReason: do not store this whole body"
            ),
            "rm file"
        );
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\nReason: rotate the token\nReply !approve to execute"
            ),
            "rotate the token"
        );
        let long = format!("```\n{}\n```", "x".repeat(300));
        assert_eq!(
            agent_approval_history_summary(&format!(
                "Approval Required: Dangerous Command\n{long}"
            ))
            .chars()
            .count(),
            AGENT_APPROVAL_HISTORY_SUMMARY_MAX_CHARS
        );
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\n```\necho visible\nexport TOKEN=secret\n```"
            ),
            "echo visible"
        );
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\n```\nls\u{2028}cat /secrets\n```"
            ),
            "ls"
        );
        let spoofed = agent_approval_history_summary(
            "Approval Required: Dangerous Command\n```\nrm \u{202E}elif\u{200B}secret\n```",
        );
        assert_eq!(spoofed, "rm elifsecret");
        assert!(!spoofed.contains('\u{202E}'));
        assert!(!spoofed.contains('\u{200B}'));
        assert!(agent_approval_history_summary(
            "Approval Required: Dangerous Command\n```\n\u{0000}token\n```"
        )
        .chars()
        .all(|ch| !ch.is_control()));
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\n```\nexport TOKEN=secret\n```"
            ),
            "export TOKEN=<redacted>"
        );
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\n```\ncurl --token abcdef\n```"
            ),
            "curl --token <redacted>"
        );
        assert_eq!(
            agent_approval_history_summary(
                "Approval Required: Dangerous Command\n```\nrm file\n```"
            ),
            "rm file"
        );
    }

    const HERMES_FORGE_PROMPT: &str = "⚠️ **Dangerous command requires approval**\nReason: find -delete\n\n```\nHOME=/home/nepenthe HERMES_HOME=/home/nepenthe/.hermes/profiles/forge /home/nepenthe/.hermes/hermes-agent/venv/bin/python -m hermes_cli.main approvals test -- find /tmp/forge-approval-test-dir -delete; echo EXIT:$?\n```\n\nReply `!approve session` for this session, `!approve always` permanently. `!approve` once · `!deny` cancel.\n\nReactions: ✅ once · 🌀 session · ♾️ always · ❌ deny";

    #[test]
    fn notification_summary_shows_reason_and_command_with_paths() {
        assert!(is_agent_approval_prompt(HERMES_FORGE_PROMPT));
        let summary = agent_approval_notification_summary(HERMES_FORGE_PROMPT);
        assert_eq!(summary.reason.as_deref(), Some("find -delete"));
        assert_eq!(
            summary.command.as_deref(),
            Some(
                "/home/nepenthe/.hermes/hermes-agent/venv/bin/python -m hermes_cli.main approvals test -- find /tmp/forge-approval-test-dir -delete; echo EXIT:$?"
            )
        );
        assert_eq!(
            agent_approval_notification_summary(HERMES_MATRIX_PROMPT),
            AgentApprovalNotificationSummary {
                reason: Some("dangerous".to_owned()),
                command: Some("rm -rf /tmp/test".to_owned()),
            }
        );
    }

    #[test]
    fn notification_summary_joins_lines_and_redacts_secrets() {
        let summary = agent_approval_notification_summary(
            "Approval Required: Dangerous Command\n```bash\nexport TOKEN=secret\ncurl --token abcdef https://x.test\n\u{202E}rm -rf ./build\n```",
        );
        assert_eq!(
            summary.command.as_deref(),
            Some(
                "export TOKEN=<redacted> ↵ curl --token <redacted> https://x.test ↵ rm -rf ./build"
            )
        );
        assert_eq!(summary.reason, None);

        let labeled = agent_approval_notification_summary(
            "Approval Required: Dangerous Command\nCode\nAPI_KEY=sk-1 rm file\nReason: clean up",
        );
        assert_eq!(labeled.command.as_deref(), Some("rm file"));
        assert_eq!(labeled.reason.as_deref(), Some("clean up"));

        let long = agent_approval_notification_summary(&format!(
            "Approval Required: Dangerous Command\n```\n{}\n```",
            "x ".repeat(400)
        ));
        let command = long.command.unwrap();
        assert!(command.chars().count() <= AGENT_APPROVAL_NOTIFICATION_COMMAND_MAX_CHARS);
        assert!(command.ends_with('…'));

        assert_eq!(
            agent_approval_notification_summary(&"x".repeat(AGENT_APPROVAL_MAX_BODY_CHARS + 1)),
            AgentApprovalNotificationSummary::default()
        );
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
                HERMES,
                CURRENT_USER,
                1_000,
                1_001,
                [(key, true)],
            )
            .unwrap();
            assert_eq!(plan.status, AgentApprovalDecisionStatus::AlreadyDecided);
        }
    }

    /// iOS's NSE mirrors this rule in Swift (`SynaraAgentApprovalFreshness`)
    /// and its XCTest reads the same vectors, so the two cannot drift.
    #[test]
    fn approval_freshness_matches_shared_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/support/notification-policy-vectors.json"
        ))
        .unwrap();
        let freshness = &vectors["agentApprovalFreshness"];
        assert_eq!(freshness["ttlMs"].as_u64(), Some(AGENT_APPROVAL_TTL_MS));
        assert_eq!(freshness["futureToleranceMs"].as_u64(), Some(60_000));
        let now_ms = freshness["nowMs"].as_u64().unwrap();
        let cases = freshness["cases"].as_array().unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let origin = case["originServerTs"].as_u64().unwrap();
            let fresh = classify_agent_approval(PROMPT, HERMES, CURRENT_USER, origin, now_ms, [])
                .is_ok_and(|classification| !classification.expired);
            assert_eq!(fresh, case["fresh"].as_bool().unwrap(), "{name}");
        }
    }
}
