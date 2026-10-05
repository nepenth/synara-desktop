//! Account-synced, explicit-sender agent notification policy. Defaults notify.
use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
        UserId,
    },
    Client,
};
use serde::{Deserialize, Serialize};

pub const AGENT_NOTIFICATION_PREFERENCES_EVENT_TYPE: &str =
    "in.synara.agent_notification_preferences";
pub const MAX_AGENT_USER_IDS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentNotificationPreferences {
    pub schema_version: u32,
    pub agent_user_ids: Vec<String>,
    pub notify_tool_activity: bool,
    pub notify_commentary: bool,
    pub notify_final_responses: bool,
}
impl Default for AgentNotificationPreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            agent_user_ids: vec![],
            notify_tool_activity: true,
            notify_commentary: true,
            notify_final_responses: true,
        }
    }
}
impl AgentNotificationPreferences {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1
            || self.agent_user_ids.len() > MAX_AGENT_USER_IDS
            || self
                .agent_user_ids
                .iter()
                .any(|id| id.len() > 255 || UserId::parse(id).is_err())
        {
            return Err("agent-notification-preferences-invalid");
        }
        Ok(())
    }
    /// Approval prompts are exempt even before the caller's action/freshness validation.
    pub fn suppresses(&self, sender: &str, body: &str) -> bool {
        if !self.agent_user_ids.iter().any(|id| id == sender)
            || crate::app::agent_approvals::is_agent_approval_prompt(body)
        {
            return false;
        }
        match classify_agent_message(body) {
            AgentMessageCategory::ToolActivity => !self.notify_tool_activity,
            AgentMessageCategory::Commentary => !self.notify_commentary,
            AgentMessageCategory::FinalResponse => !self.notify_final_responses,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMessageCategory {
    ToolActivity,
    Commentary,
    FinalResponse,
}
/// Recognize only a first-line Hermes summary heading followed by content.
/// Quoted/code-fenced headings, incidental keywords and malformed counts are prose.
pub fn classify_agent_message(body: &str) -> AgentMessageCategory {
    let mut lines = body.trim_start().lines();
    let mut heading = lines.next().unwrap_or("").trim();
    if let Some(value) = heading
        .strip_prefix("# ")
        .or_else(|| heading.strip_prefix("## "))
        .or_else(|| heading.strip_prefix("### "))
    {
        heading = value;
    }
    if let Some(value) = heading
        .strip_prefix("**")
        .and_then(|s| s.strip_suffix("**"))
    {
        heading = value;
    }
    let category = if let Some(count) = heading.strip_prefix("🛠 Tool activity (") {
        Some((AgentMessageCategory::ToolActivity, count))
    } else {
        heading
            .strip_prefix("💬 Commentary (")
            .map(|count| (AgentMessageCategory::Commentary, count))
    };
    let Some((category, count)) = category else {
        return AgentMessageCategory::FinalResponse;
    };
    let Some(number) = count
        .strip_suffix(" updates)")
        .or_else(|| count.strip_suffix(" update)"))
    else {
        return AgentMessageCategory::FinalResponse;
    };
    if number.is_empty()
        || !number.bytes().all(|c| c.is_ascii_digit())
        || number.parse::<u32>().ok().filter(|n| *n > 0).is_none()
    {
        return AgentMessageCategory::FinalResponse;
    }
    if !lines.any(|line| !line.trim().is_empty()) {
        return AgentMessageCategory::FinalResponse;
    }
    category
}
fn event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(AGENT_NOTIFICATION_PREFERENCES_EVENT_TYPE)
}
fn parse(
    raw: Option<Raw<AnyGlobalAccountDataEventContent>>,
) -> Result<AgentNotificationPreferences, &'static str> {
    let Some(raw) = raw else {
        return Ok(AgentNotificationPreferences::default());
    };
    if raw.json().get().len() > 40_000 {
        return Err("agent-notification-preferences-invalid");
    }
    let value: AgentNotificationPreferences = raw
        .deserialize_as_unchecked()
        .map_err(|_| "agent-notification-preferences-invalid")?;
    value.validate()?;
    Ok(value)
}
pub async fn cached_agent_notification_preferences(
    client: &Client,
) -> Result<AgentNotificationPreferences, &'static str> {
    parse(
        client
            .account()
            .account_data_raw(event_type())
            .await
            .map_err(|_| "agent-notification-preferences-load-failed")?,
    )
}
pub async fn fetch_agent_notification_preferences(
    client: &Client,
) -> Result<AgentNotificationPreferences, &'static str> {
    parse(
        client
            .account()
            .fetch_account_data(event_type())
            .await
            .map_err(|_| "agent-notification-preferences-load-failed")?,
    )
}
pub async fn store_agent_notification_preferences(
    client: &Client,
    preferences: &AgentNotificationPreferences,
) -> Result<(), &'static str> {
    preferences.validate()?;
    let raw = Raw::from_json(
        serde_json::value::to_raw_value(preferences)
            .map_err(|_| "agent-notification-preferences-invalid")?,
    );
    client
        .account()
        .set_account_data_raw(event_type(), raw)
        .await
        .map_err(|_| "agent-notification-preferences-save-failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_hermes_headers_and_prose_boundaries() {
        for (body, category) in [
            (
                "**🛠 Tool activity (7 updates)**\n\n1. 💻 terminal",
                AgentMessageCategory::ToolActivity,
            ),
            (
                "  **💬 Commentary (1 update)**\n\n1. I’ll confirm",
                AgentMessageCategory::Commentary,
            ),
            (
                "### 💬 Commentary (2 updates)\n1. Hello",
                AgentMessageCategory::Commentary,
            ),
        ] {
            assert_eq!(classify_agent_message(body), category);
        }
        for body in [
            "Yes. The vLLM answers are saved.",
            "Here is **🛠 Tool activity (7 updates)**\n1. foo",
            "> **💬 Commentary (1 update)**\n1. hello",
            "```\n**💬 Commentary (1 update)**\n1. hello",
            "**💬 Commentary (0 updates)**\n1. hello",
        ] {
            assert_eq!(
                classify_agent_message(body),
                AgentMessageCategory::FinalResponse
            );
        }
    }
    #[test]
    fn explicit_senders_defaults_and_category_controls() {
        let mut p = AgentNotificationPreferences::default();
        let body = "**🛠 Tool activity (7 updates)**\n1. terminal";
        assert!(!p.suppresses("@forge:example.org", body));
        p.agent_user_ids.push("@forge:example.org".into());
        p.notify_tool_activity = false;
        assert!(p.suppresses("@forge:example.org", body));
        assert!(!p.suppresses("@human:example.org", body));
        assert!(!p.suppresses("@forge:example.org", "Final answer"));
        p.notify_final_responses = false;
        assert!(p.suppresses("@forge:example.org", "Final answer"));
    }
    #[test]
    fn every_approval_heading_bypasses_all_disabled_categories() {
        let p = AgentNotificationPreferences {
            agent_user_ids: vec!["@forge:example.org".into()],
            notify_tool_activity: false,
            notify_commentary: false,
            notify_final_responses: false,
            ..Default::default()
        };
        for body in [
            "Approval Required: Dangerous Command\nCode\nrm file",
            "⚠️ **Dangerous command requires approval**\n```\nrm file\n```",
        ] {
            assert!(!p.suppresses("@forge:example.org", body));
        }
    }
    #[test]
    fn invalid_or_future_account_data_is_never_silently_rewritten() {
        assert_eq!(
            parse(None).unwrap(),
            AgentNotificationPreferences::default()
        );
        let mut p = AgentNotificationPreferences {
            schema_version: 2,
            ..Default::default()
        };
        assert!(p.validate().is_err());
        p.schema_version = 1;
        p.agent_user_ids.push("Forge".into());
        assert!(p.validate().is_err());
    }
}
