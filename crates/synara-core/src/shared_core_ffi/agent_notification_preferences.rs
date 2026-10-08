use super::SharedCore;
use crate::app::notifications::AgentNotificationPreferences;
use crate::transport::MatrixIpcError;
pub type AgentNotificationPreferencesDto = AgentNotificationPreferences;
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
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
    async fn agent_preferences_command(
        &self,
        request: impl std::future::Future<Output = Result<AgentNotificationPreferences, MatrixIpcError>>,
    ) -> Result<AgentNotificationPreferencesDto, AgentNotificationPreferencesError> {
        let response = request
            .await
            .map_err(|e| AgentNotificationPreferencesError::Failed {
                code: e
                    .diagnostic_id
                    .unwrap_or_else(|| "agent-notification-preferences-load-failed".into()),
                description: "Agent notification settings are unavailable.".into(),
            })?;
        Ok(response)
    }
}

#[uniffi::export(async_runtime = "tokio")]
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
    pub async fn agent_notification_preferences_snapshot(
        &self,
    ) -> Result<AgentNotificationPreferencesDto, AgentNotificationPreferencesError> {
        self.agent_preferences_command(self.core.agent_notification_preferences_snapshot())
            .await
    }
    pub async fn agent_notification_preferences_set(
        &self,
        preferences: AgentNotificationPreferencesDto,
    ) -> Result<AgentNotificationPreferencesDto, AgentNotificationPreferencesError> {
        self.agent_preferences_command(self.core.agent_notification_preferences_set(
            crate::core_api::AgentPreferencesSetRequest { preferences },
        ))
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
