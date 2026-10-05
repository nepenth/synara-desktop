use super::SharedCore;
use crate::app::notifications::AgentNotificationPreferences;
use crate::transport::CommandEnvelope;
pub type AgentNotificationPreferencesDto = AgentNotificationPreferences;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentNotificationPreferencesError {
    Failed { code: String, description: String },
}
impl std::fmt::Display for AgentNotificationPreferencesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => f.write_str(description),
        }
    }
}
impl std::error::Error for AgentNotificationPreferencesError {}
impl SharedCore {
    pub async fn agent_notification_event_allowed(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<bool, AgentNotificationPreferencesError> {
        self.core
            .agent_notification_event_allowed(&room_id, &event_id)
            .await
            .map_err(|e| AgentNotificationPreferencesError::Failed {
                code: e
                    .diagnostic_id
                    .unwrap_or_else(|| "agent-notification-event-unavailable".into()),
                description: "Notification policy could not be resolved.".into(),
            })
    }

    async fn agent_preferences_command(
        &self,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<AgentNotificationPreferencesDto, AgentNotificationPreferencesError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: name.into(),
                session_generation: 0,
                request_id: None,
                payload,
            })
            .await
            .map_err(|e| AgentNotificationPreferencesError::Failed {
                code: e
                    .diagnostic_id
                    .unwrap_or_else(|| "agent-notification-preferences-load-failed".into()),
                description: "Agent notification settings are unavailable.".into(),
            })?;
        serde_json::from_value(response.payload).map_err(|_| {
            AgentNotificationPreferencesError::Failed {
                code: "agent-notification-preferences-invalid".into(),
                description: "Agent notification settings are invalid.".into(),
            }
        })
    }
    pub async fn agent_notification_preferences_snapshot(
        &self,
    ) -> Result<AgentNotificationPreferencesDto, AgentNotificationPreferencesError> {
        self.agent_preferences_command(
            "matrix_agent_notification_preferences_snapshot",
            serde_json::Value::Null,
        )
        .await
    }
    pub async fn agent_notification_preferences_set(
        &self,
        preferences: AgentNotificationPreferencesDto,
    ) -> Result<AgentNotificationPreferencesDto, AgentNotificationPreferencesError> {
        self.agent_preferences_command(
            "matrix_agent_notification_preferences_set",
            serde_json::json!({"preferences":preferences}),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn agent_preferences_methods_without_session_fail_closed() {
        let core = SharedCore::new();
        for result in [
            core.agent_notification_preferences_snapshot().await,
            core.agent_notification_preferences_set(Default::default())
                .await,
        ] {
            assert!(
                matches!(result,Err(AgentNotificationPreferencesError::Failed{code,..}) if code == "agent-notification-preferences-no-session")
            );
        }
        assert!(
            matches!(core.agent_notification_event_allowed("!room:example.org".into(),"$event".into()).await,Err(AgentNotificationPreferencesError::Failed{code,..}) if code=="agent-notification-preferences-no-session")
        );
    }
}
