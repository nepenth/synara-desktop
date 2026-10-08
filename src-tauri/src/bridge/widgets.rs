//! Desktop bridge for experimental widget commands through `Core::command`.

use synara_core::app::widgets::{
    AgentWidgetEntry, ListedWidget, WidgetKind, WidgetListSnapshot, WidgetOpenResult,
    WidgetSessionRecord,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn widgets_list(
    core: &Core,
    experimental_widgets_enabled: bool,
    room_id: String,
    agent_widgets: Vec<AgentWidgetEntry>,
) -> Result<WidgetListSnapshot, MatrixAuthCommandError> {
    let response = core
        .widgets_list(synara_core::core_api::MatrixWidgetsListRequest {
            experimental_widgets_enabled,
            room_id,
            agent_widgets,
        })
        .await
        .map_err(map_widget_core_error)?;
    Ok(response)
}

#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub(crate) async fn widget_open(
    core: &Core,
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
    let response = core
        .widget_open(synara_core::core_api::MatrixWidgetOpenRequest {
            experimental_widgets_enabled,
            room_id,
            widget_id,
            name,
            url,
            kind,
            init_on_content_load,
            receive_room,
            send_room_message,
        })
        .await
        .map_err(map_widget_core_error)?;
    Ok(response)
}

pub(crate) async fn widget_close(
    core: &Core,
    session_id: Option<String>,
) -> Result<Vec<String>, MatrixAuthCommandError> {
    let response = core
        .widget_close(synara_core::core_api::MatrixWidgetCloseRequest { session_id })
        .await
        .map_err(map_widget_core_error)?;
    Ok(response)
}

pub(crate) async fn widget_post(
    core: &Core,
    experimental_widgets_enabled: bool,
    session_id: String,
    message: String,
) -> Result<(), MatrixAuthCommandError> {
    core.widget_post(synara_core::core_api::MatrixWidgetPostRequest {
        experimental_widgets_enabled,
        session_id,
        message,
    })
    .await
    .map_err(map_widget_core_error)?;
    Ok(())
}

pub(crate) async fn widget_subscribe(
    core: &Core,
    experimental_widgets_enabled: bool,
) -> Result<Vec<WidgetSessionRecord>, MatrixAuthCommandError> {
    let response = core
        .widget_subscribe(synara_core::core_api::MatrixWidgetSubscribeRequest {
            experimental_widgets_enabled,
        })
        .await
        .map_err(map_widget_core_error)?;
    Ok(response)
}

fn map_widget_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.diagnostic_id.as_deref().unwrap_or("") {
        diagnostic @ ("experimental-widgets-disabled"
        | "p2-widgets-list-no-session"
        | "p2-widget-open-no-session"
        | "p2-widget-close-no-session"
        | "p2-widget-post-no-session"
        | "p2-widget-subscribe-no-session"
        | "experimental-widgets-session-not-live") => MatrixAuthCommandError::new(
            "Forbidden",
            "Experimental widgets are unavailable.",
            diagnostic,
        ),
        diagnostic @ ("experimental-widgets-url-rejected"
        | "experimental-widgets-room-state-url-rejected"
        | "experimental-widgets-invalid-room"
        | "experimental-widgets-room-missing"
        | "experimental-widgets-session-missing"
        | "experimental-widgets-registry-full"
        | "p2-widgets-list-invalid-payload"
        | "p2-widget-open-invalid-payload"
        | "p2-widget-close-invalid-payload"
        | "p2-widget-post-invalid-payload"
        | "p2-widget-subscribe-invalid-payload") => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix widget request is invalid.",
            diagnostic,
        ),
        diagnostic => {
            let code = match error.category {
                MatrixIpcErrorCategory::Forbidden => "Forbidden",
                MatrixIpcErrorCategory::SdkInvariant => "InvalidRequest",
                MatrixIpcErrorCategory::StaleSessionGeneration => "StaleSessionGeneration",
                _ => "Unknown",
            };
            MatrixAuthCommandError::new(
                code,
                "Native Matrix widgets are unavailable.",
                if diagnostic.is_empty() {
                    "experimental-widgets-unavailable"
                } else {
                    diagnostic
                },
            )
        }
    }
}

#[allow(dead_code)]
pub(crate) fn _listed_widget_type(_: ListedWidget) {}
