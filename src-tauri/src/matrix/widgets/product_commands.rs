use super::*;
use crate::matrix::widgets::host::{
    open_widget_window, session_id_from_widget_label, WIDGET_WINDOW_PREFIX,
};
use crate::matrix::widgets::{
    AgentWidgetEntry, NativeWidgetOwner, WidgetKind, WidgetListSnapshot, WidgetOpenResult,
    WidgetSessionRecord,
};
use tauri::WebviewWindow;

#[tauri::command]
pub async fn matrix_widgets_list(
    core: State<'_, Arc<synara_core::Core>>,
    experimental_widgets_enabled: bool,
    room_id: String,
    agent_widgets: Vec<AgentWidgetEntry>,
) -> Result<WidgetListSnapshot, MatrixAuthCommandError> {
    crate::bridge::widgets::widgets_list(
        core.inner().as_ref(),
        experimental_widgets_enabled,
        room_id,
        agent_widgets,
    )
    .await
}

#[tauri::command]
pub async fn matrix_widget_open(
    app: AppHandle,
    core: State<'_, Arc<synara_core::Core>>,
    experimental_widgets_enabled: bool,
    room_id: String,
    widget_id: String,
    name: String,
    url: String,
    kind: WidgetKind,
    init_on_content_load: bool,
    receive_room: bool,
    send_room_message: bool,
) -> Result<WidgetOpenResult, MatrixAuthCommandError> {
    let allow_loopback = matches!(kind, WidgetKind::Agent);
    let opened = crate::bridge::widgets::widget_open(
        core.inner().as_ref(),
        experimental_widgets_enabled,
        room_id,
        widget_id,
        name,
        url,
        kind,
        init_on_content_load,
        receive_room,
        send_room_message,
    )
    .await?;
    if let Err(error) = open_widget_window(&app, &opened, allow_loopback) {
        let _ = crate::bridge::widgets::widget_close(
            core.inner().as_ref(),
            Some(opened.session_id.clone()),
        )
        .await;
        return Err(error);
    }
    Ok(opened)
}

#[tauri::command]
pub async fn matrix_widget_close(
    core: State<'_, Arc<synara_core::Core>>,
    session_id: Option<String>,
) -> Result<Vec<String>, MatrixAuthCommandError> {
    crate::bridge::widgets::widget_close(core.inner().as_ref(), session_id).await
}

#[tauri::command]
pub async fn matrix_widget_post(
    core: State<'_, Arc<synara_core::Core>>,
    experimental_widgets_enabled: bool,
    session_id: String,
    message: String,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::widgets::widget_post(
        core.inner().as_ref(),
        experimental_widgets_enabled,
        session_id,
        message,
    )
    .await
}

#[tauri::command]
pub async fn matrix_widget_subscribe(
    core: State<'_, Arc<synara_core::Core>>,
    experimental_widgets_enabled: bool,
) -> Result<Vec<WidgetSessionRecord>, MatrixAuthCommandError> {
    crate::bridge::widgets::widget_subscribe(core.inner().as_ref(), experimental_widgets_enabled)
        .await
}

/// Isolated widget webview → driver pump. Session id is taken from the window
/// label so a widget cannot post into another widget's driver.
#[tauri::command]
pub async fn widget_bridge_post(
    window: WebviewWindow,
    core: State<'_, Arc<synara_core::Core>>,
    message: String,
) -> Result<(), MatrixAuthCommandError> {
    let Some(session_id) = session_id_from_widget_label(window.label()) else {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "Experimental widgets are unavailable.",
            "experimental-widgets-bridge-window-rejected",
        ));
    };
    if window.label() == crate::desktop::MAIN_WINDOW_LABEL
        || !window.label().starts_with(WIDGET_WINDOW_PREFIX)
    {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "Experimental widgets are unavailable.",
            "experimental-widgets-bridge-window-rejected",
        ));
    }
    crate::bridge::widgets::widget_post(core.inner().as_ref(), true, session_id.to_owned(), message)
        .await
}

pub(crate) fn map_widget_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "experimental-widgets-disabled"
        | "experimental-widgets-session-not-live"
        | "experimental-widgets-owner-missing" => {
            ("Forbidden", "Experimental widgets are unavailable.")
        }
        "experimental-widgets-url-rejected"
        | "experimental-widgets-room-state-url-rejected"
        | "experimental-widgets-invalid-room"
        | "experimental-widgets-room-missing"
        | "experimental-widgets-session-missing"
        | "experimental-widgets-registry-full"
        | "experimental-widgets-driver-stopped"
        | "experimental-widgets-state-read-failed" => (
            "InvalidRequest",
            "The native Matrix widget request is invalid.",
        ),
        _ => ("Unknown", "Native Matrix widgets are unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

#[allow(dead_code)]
pub(super) fn start_widget_owner_for_session(
    client: &matrix_sdk::Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<Arc<NativeWidgetOwner>, MatrixAuthCommandError> {
    crate::matrix::widgets::start_widget_owner(client, app, session_generation)
        .map(Arc::new)
        .map_err(map_widget_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_bridge_is_not_a_matrix_command() {
        assert!(!stringify!(widget_bridge_post).starts_with("matrix_"));
    }

    #[test]
    fn lifecycle_diagnostics_use_privacy_safe_categories() {
        let cases = [
            ("experimental-widgets-disabled", "Forbidden"),
            ("experimental-widgets-session-not-live", "Forbidden"),
            ("experimental-widgets-url-rejected", "InvalidRequest"),
            (
                "experimental-widgets-room-state-url-rejected",
                "InvalidRequest",
            ),
            ("experimental-widgets-unavailable", "Unknown"),
        ];
        for (diagnostic_id, expected_code) in cases {
            let error = map_widget_error(diagnostic_id);
            assert_eq!(error.code, expected_code);
            assert_eq!(error.diagnostic_id, diagnostic_id);
            assert!(!error.message.contains("@"));
            assert!(!error.message.contains("token"));
        }
    }
}
