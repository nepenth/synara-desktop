use crate::matrix::auth::product::MatrixAuthCommandError;
use std::sync::Arc;
use synara_core::{app::notifications::AgentNotificationPreferences, Core};
use tauri::State;
#[tauri::command]
pub async fn matrix_agent_notification_preferences_snapshot(
    core: State<'_, Arc<Core>>,
) -> Result<AgentNotificationPreferences, MatrixAuthCommandError> {
    core.agent_notification_preferences_snapshot()
        .await
        .map_err(map_agent_preferences_error)
}
#[tauri::command]
pub async fn matrix_agent_notification_preferences_set(
    core: State<'_, Arc<Core>>,
    preferences: AgentNotificationPreferences,
) -> Result<AgentNotificationPreferences, MatrixAuthCommandError> {
    core.agent_notification_preferences_set(synara_core::core_api::AgentPreferencesSetRequest {
        preferences,
    })
    .await
    .map_err(map_agent_preferences_error)
}

fn map_agent_preferences_error(
    error: synara_core::transport::MatrixIpcError,
) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Agent notification settings are unavailable.",
        error
            .diagnostic_id
            .as_deref()
            .unwrap_or("agent-notification-preferences-load-failed"),
    )
}
