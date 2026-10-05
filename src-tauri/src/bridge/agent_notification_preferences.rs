use crate::matrix::auth::product::MatrixAuthCommandError;
use std::sync::Arc;
use synara_core::{
    app::notifications::AgentNotificationPreferences, transport::CommandEnvelope, Core,
};
use tauri::State;
async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<AgentNotificationPreferences, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.into(),
            session_generation: 0,
            request_id: None,
            payload,
        })
        .await
        .map_err(|e| {
            MatrixAuthCommandError::new(
                "Unknown",
                "Agent notification settings are unavailable.",
                e.diagnostic_id
                    .as_deref()
                    .unwrap_or("agent-notification-preferences-load-failed"),
            )
        })?;
    serde_json::from_value(response.payload).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "Agent notification settings are invalid.",
            "agent-notification-preferences-invalid",
        )
    })
}
#[tauri::command]
pub async fn matrix_agent_notification_preferences_snapshot(
    core: State<'_, Arc<Core>>,
) -> Result<AgentNotificationPreferences, MatrixAuthCommandError> {
    dispatch(
        core.inner().as_ref(),
        "matrix_agent_notification_preferences_snapshot",
        serde_json::Value::Null,
    )
    .await
}
#[tauri::command]
pub async fn matrix_agent_notification_preferences_set(
    core: State<'_, Arc<Core>>,
    preferences: AgentNotificationPreferences,
) -> Result<AgentNotificationPreferences, MatrixAuthCommandError> {
    dispatch(
        core.inner().as_ref(),
        "matrix_agent_notification_preferences_set",
        serde_json::json!({"preferences":preferences}),
    )
    .await
}
