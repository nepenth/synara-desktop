//! Desktop bridge for experimental widget commands through `Core::command`.

use synara_core::app::widgets::{
    AgentWidgetEntry, ListedWidget, WidgetKind, WidgetListSnapshot, WidgetOpenResult,
    WidgetSessionRecord,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const WIDGETS_LIST_COMMAND: &str = "matrix_widgets_list";
const WIDGET_OPEN_COMMAND: &str = "matrix_widget_open";
const WIDGET_CLOSE_COMMAND: &str = "matrix_widget_close";
const WIDGET_POST_COMMAND: &str = "matrix_widget_post";
const WIDGET_SUBSCRIBE_COMMAND: &str = "matrix_widget_subscribe";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn widgets_list(
    core: &Core,
    experimental_widgets_enabled: bool,
    room_id: String,
    agent_widgets: Vec<AgentWidgetEntry>,
) -> Result<WidgetListSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: WIDGETS_LIST_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "experimentalWidgetsEnabled": experimental_widgets_enabled,
                "roomId": room_id,
                "agentWidgets": agent_widgets,
            }),
        })
        .await
        .map_err(map_widget_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| widget_response_error())
}

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
        .command(CommandEnvelope {
            command: WIDGET_OPEN_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "experimentalWidgetsEnabled": experimental_widgets_enabled,
                "roomId": room_id,
                "widgetId": widget_id,
                "name": name,
                "url": url,
                "kind": kind,
                "initOnContentLoad": init_on_content_load,
                "receiveRoom": receive_room,
                "sendRoomMessage": send_room_message,
            }),
        })
        .await
        .map_err(map_widget_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| widget_response_error())
}

pub(crate) async fn widget_close(
    core: &Core,
    session_id: Option<String>,
) -> Result<Vec<String>, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: WIDGET_CLOSE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "sessionId": session_id }),
        })
        .await
        .map_err(map_widget_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| widget_response_error())
}

pub(crate) async fn widget_post(
    core: &Core,
    experimental_widgets_enabled: bool,
    session_id: String,
    message: String,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: WIDGET_POST_COMMAND.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({
            "experimentalWidgetsEnabled": experimental_widgets_enabled,
            "sessionId": session_id,
            "message": message,
        }),
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
        .command(CommandEnvelope {
            command: WIDGET_SUBSCRIBE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "experimentalWidgetsEnabled": experimental_widgets_enabled,
            }),
        })
        .await
        .map_err(map_widget_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| widget_response_error())
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

fn widget_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix widgets are unavailable.",
        "experimental-widgets-unavailable",
    )
}

#[allow(dead_code)]
pub(crate) fn _listed_widget_type(_: ListedWidget) {}
