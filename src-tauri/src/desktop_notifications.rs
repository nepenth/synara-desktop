use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::desktop::navigate_main_window;
#[cfg(test)]
use crate::desktop_sanitize::sanitize_route;
use crate::desktop_sanitize::{sanitize_action_text, sanitize_notification_route};

const DESKTOP_NOTIFICATION_MAX_TITLE_CHARS: usize = 120;
const DESKTOP_NOTIFICATION_MAX_BODY_CHARS: usize = 500;
const DESKTOP_NOTIFICATION_MAX_ACTIONS: usize = 4;
const DESKTOP_NOTIFICATION_MAX_ACTION_ID_CHARS: usize = 96;
const DESKTOP_NOTIFICATION_MAX_ACTION_LABEL_CHARS: usize = 80;
const DESKTOP_NOTIFICATION_MAX_ACTION_CONTEXT_CHARS: usize = 255;
const DESKTOP_NOTIFICATION_DEFAULT_ACTION_ID: &str = "default";
const DESKTOP_NOTIFICATION_ACTION_EVENT: &str = "synara://notification-action";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationAction {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationActionContext {
    pub kind: String,
    pub room_id: Option<String>,
    pub event_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopNotificationActionEvent {
    action_id: String,
    context: Option<DesktopNotificationActionContext>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationPayload {
    pub title: String,
    pub body: Option<String>,
    pub route: Option<String>,
    pub actions: Option<Vec<DesktopNotificationAction>>,
    pub action_context: Option<DesktopNotificationActionContext>,
}

fn sanitize_notification_payload(
    notification: DesktopNotificationPayload,
) -> Result<DesktopNotificationPayload, String> {
    let title = sanitize_action_text(notification.title, DESKTOP_NOTIFICATION_MAX_TITLE_CHARS);
    if title.is_empty() {
        return Err("Notification title cannot be empty".to_owned());
    }

    let body = notification
        .body
        .map(|value| sanitize_action_text(value, DESKTOP_NOTIFICATION_MAX_BODY_CHARS))
        .filter(|value| !value.is_empty());

    let route = match notification.route {
        Some(value) => Some(sanitize_notification_route(value)?),
        None => None,
    };
    let actions = sanitize_notification_actions(notification.actions);
    let action_context = notification
        .action_context
        .and_then(sanitize_notification_action_context);

    Ok(DesktopNotificationPayload {
        title,
        body,
        route,
        actions,
        action_context,
    })
}

fn is_safe_action_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-'))
}

fn sanitize_notification_actions(
    actions: Option<Vec<DesktopNotificationAction>>,
) -> Option<Vec<DesktopNotificationAction>> {
    let mut sanitized = Vec::new();

    for action in actions.unwrap_or_default().into_iter() {
        if sanitized.len() >= DESKTOP_NOTIFICATION_MAX_ACTIONS {
            break;
        }

        let id = sanitize_action_text(action.id, DESKTOP_NOTIFICATION_MAX_ACTION_ID_CHARS);
        let label = sanitize_action_text(action.label, DESKTOP_NOTIFICATION_MAX_ACTION_LABEL_CHARS);
        if id.is_empty()
            || label.is_empty()
            || !is_safe_action_id(&id)
            || id == DESKTOP_NOTIFICATION_DEFAULT_ACTION_ID
            || sanitized
                .iter()
                .any(|existing: &DesktopNotificationAction| {
                    existing.id == id || existing.label == label
                })
        {
            continue;
        }

        sanitized.push(DesktopNotificationAction { id, label });
    }

    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

fn sanitize_notification_action_context(
    context: DesktopNotificationActionContext,
) -> Option<DesktopNotificationActionContext> {
    let kind = sanitize_action_text(
        context.kind.to_lowercase(),
        DESKTOP_NOTIFICATION_MAX_ACTION_CONTEXT_CHARS,
    );
    if kind.is_empty() {
        return None;
    }

    let room_id = context
        .room_id
        .map(|value| sanitize_action_text(value, DESKTOP_NOTIFICATION_MAX_ACTION_CONTEXT_CHARS))
        .filter(|value| !value.is_empty());
    let event_id = context
        .event_id
        .map(|value| sanitize_action_text(value, DESKTOP_NOTIFICATION_MAX_ACTION_CONTEXT_CHARS))
        .filter(|value| !value.is_empty());

    Some(DesktopNotificationActionContext {
        kind,
        room_id,
        event_id,
    })
}

fn emit_notification_action<R: Runtime>(
    app: &AppHandle<R>,
    action_id: &str,
    context: Option<DesktopNotificationActionContext>,
) -> Result<(), String> {
    app.emit(
        DESKTOP_NOTIFICATION_ACTION_EVENT,
        DesktopNotificationActionEvent {
            action_id: action_id.to_owned(),
            context,
        },
    )
    .map_err(|error| error.to_string())
}

fn is_time_sensitive_agent_approval(
    context: Option<&DesktopNotificationActionContext>,
) -> bool {
    context.is_some_and(|context| context.kind == "agent-approval")
}

fn show_notification_without_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
) -> Result<(), String> {
    let mut builder = app.notification().builder().title(title.to_owned());
    if let Some(body) = body {
        builder = builder.body(body.to_owned());
    }
    builder.show().map_err(|error| error.to_string())
}

#[cfg(target_os = "linux")]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: Option<&str>,
    actions: &[DesktopNotificationAction],
    action_context: Option<&DesktopNotificationActionContext>,
) -> Result<(), String> {
    use notify_rust::Notification;
    use notify_rust::Urgency;

    let mut notification = Notification::new();
    notification.summary(title);
    if let Some(body) = body {
        notification.body(body);
    }
    notification.auto_icon();
    if is_time_sensitive_agent_approval(action_context) {
        notification.urgency(Urgency::Critical);
        notification.timeout(notify_rust::Timeout::Milliseconds(300_000));
    }
    if route.is_some() {
        notification.action(DESKTOP_NOTIFICATION_DEFAULT_ACTION_ID, "Open Synara");
    }
    for action in actions {
        notification.action(&action.id, &action.label);
    }

    let handle = notification.show().map_err(|error| error.to_string())?;
    let app = app.clone();
    let route = route.map(str::to_owned);
    let action_context = action_context.cloned();
    let allowed_action_ids = actions
        .iter()
        .map(|action| action.id.clone())
        .collect::<Vec<_>>();

    tauri::async_runtime::spawn(async move {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            handle.wait_for_action(move |action| {
                if action == DESKTOP_NOTIFICATION_DEFAULT_ACTION_ID {
                    let Some(route) = route.as_deref() else {
                        return;
                    };
                    if let Err(error) = navigate_main_window(&app, route) {
                        eprintln!("failed to navigate from notification click: {error}");
                    }
                    return;
                }

                if allowed_action_ids
                    .iter()
                    .any(|candidate| candidate == action)
                {
                    if let Err(error) =
                        emit_notification_action(&app, action, action_context.clone())
                    {
                        eprintln!("failed to emit notification action: {error}");
                    }
                }
            });
        })
        .await;
    });

    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_macos_notification_application() {
    use mac_notification_sys::set_application;

    let bundle_identifier = if tauri::is_dev() {
        "com.apple.Terminal"
    } else {
        "com.whylandcreative.synara.desktop"
    };

    if let Err(error) = set_application(bundle_identifier) {
        eprintln!("failed to configure macOS notification application: {error}");
    }
}

#[cfg(target_os = "macos")]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: Option<&str>,
    actions: &[DesktopNotificationAction],
    action_context: Option<&DesktopNotificationActionContext>,
) -> Result<(), String> {
    use mac_notification_sys::{MainButton, Notification, NotificationResponse, Sound};

    configure_macos_notification_application();

    let title = title.to_owned();
    let body = body.map(str::to_owned);
    let app = app.clone();
    let route = route.map(str::to_owned);
    let action_context = action_context.cloned();
    let time_sensitive = is_time_sensitive_agent_approval(action_context.as_ref());
    let action_labels = actions
        .iter()
        .map(|action| action.label.clone())
        .collect::<Vec<_>>();
    let action_ids_by_label = actions
        .iter()
        .map(|action| (action.label.clone(), action.id.clone()))
        .collect::<Vec<_>>();

    tauri::async_runtime::spawn(async move {
        let app = app.clone();
        let route = route.clone();
        let action_context = action_context.clone();
        let response = tauri::async_runtime::spawn_blocking(move || {
            let action_label_refs = action_labels.iter().map(String::as_str).collect::<Vec<_>>();
            let mut notification = Notification::new();
            notification.title(&title);
            if let Some(ref body) = body {
                notification.message(body);
            }
            if time_sensitive {
                notification.subtitle("Time-sensitive · expires in 5 minutes");
                notification.sound(Sound::Default);
            }
            if action_labels.len() == 1 {
                notification.main_button(MainButton::SingleAction(action_labels[0].as_str()));
            } else if action_labels.len() > 1 {
                notification
                    .main_button(MainButton::DropdownActions("Respond", &action_label_refs));
            }
            notification.wait_for_click(true);
            notification.send()
        })
        .await;

        if let Ok(Ok(response)) = response {
            match response {
                NotificationResponse::Click => {
                    let Some(route) = route.as_deref() else {
                        return;
                    };
                    if let Err(error) = navigate_main_window(&app, route) {
                        eprintln!("failed to navigate from notification click: {error}");
                    }
                }
                NotificationResponse::ActionButton(label) => {
                    let Some((_, action_id)) = action_ids_by_label
                        .iter()
                        .find(|(candidate_label, _)| candidate_label == &label)
                    else {
                        return;
                    };
                    if let Err(error) =
                        emit_notification_action(&app, action_id, action_context.clone())
                    {
                        eprintln!("failed to emit notification action: {error}");
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    _route: Option<&str>,
    _actions: &[DesktopNotificationAction],
    _action_context: Option<&DesktopNotificationActionContext>,
) -> Result<(), String> {
    show_notification_without_route_click_handler(app, title, body)
}

#[tauri::command]
pub fn desktop_get_notification_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<String, String> {
    app.notification()
        .permission_state()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_request_notification_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<String, String> {
    app.notification()
        .request_permission()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_notify<R: Runtime>(
    app: AppHandle<R>,
    notification: DesktopNotificationPayload,
) -> Result<bool, String> {
    let notification = sanitize_notification_payload(notification)?;
    let actions = notification.actions.as_deref().unwrap_or(&[]);

    if notification.route.is_some() || !actions.is_empty() {
        show_notification_with_route_click_handler(
            &app,
            &notification.title,
            notification.body.as_deref(),
            notification.route.as_deref(),
            actions,
            notification.action_context.as_ref(),
        )?;
        return Ok(true);
    }

    show_notification_without_route_click_handler(
        &app,
        &notification.title,
        notification.body.as_deref(),
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_notification_payload_rejects_empty_title() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            title: "  ".to_owned(),
            body: Some("Body".to_owned()),
            route: None,
            actions: None,
            action_context: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_notification_payload_truncates_body() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("a".repeat(DESKTOP_NOTIFICATION_MAX_BODY_CHARS + 10)),
            route: Some("/inbox/".to_owned()),
            actions: None,
            action_context: None,
        })
        .expect("notification payload should pass");

        assert_eq!(
            payload.body.unwrap().chars().count(),
            DESKTOP_NOTIFICATION_MAX_BODY_CHARS
        );
    }

    #[test]
    fn sanitize_notification_route_allows_only_internal_routes() {
        assert_eq!(
            sanitize_notification_route("/inbox/later/".to_owned()).unwrap(),
            "/inbox/later/"
        );
        assert_eq!(
            sanitize_notification_route("#/room/abc".to_owned()).unwrap(),
            "#/room/abc"
        );
        let notification = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Later".to_owned(),
            body: Some("Reminder".to_owned()),
            route: Some("/inbox/later/".to_owned()),
            actions: None,
            action_context: None,
        })
        .expect("notification payload should sanitize");
        let route = notification.route.expect("route should be present");
        assert_eq!(sanitize_route(route.clone()).unwrap(), route);
        assert!(sanitize_notification_route("https://example.org".to_owned()).is_err());
        assert!(sanitize_notification_route("room/abc".to_owned()).is_err());
    }

    #[test]
    fn sanitize_notification_payload_accepts_safe_route() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("/inbox/notifications/".to_owned()),
            actions: None,
            action_context: None,
        })
        .expect("notification payload should pass");
        assert_eq!(payload.route, Some("/inbox/notifications/".to_string()));
    }

    #[test]
    fn sanitize_notification_payload_rejects_unsafe_route() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("https://evil.example.com".to_owned()),
            actions: None,
            action_context: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_notification_payload_sanitizes_actions_and_context() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Approval".to_owned(),
            body: Some("body".to_owned()),
            route: Some("/room/!room".to_owned()),
            actions: Some(vec![
                DesktopNotificationAction {
                    id: " agent-approval.approve-once ".to_owned(),
                    label: " Approve once ".to_owned(),
                },
                DesktopNotificationAction {
                    id: "agent-approval.deny".to_owned(),
                    label: "Deny".to_owned(),
                },
                DesktopNotificationAction {
                    id: "bad action id".to_owned(),
                    label: "Bad".to_owned(),
                },
            ]),
            action_context: Some(DesktopNotificationActionContext {
                kind: " Agent-Approval ".to_owned(),
                room_id: Some(" !room:matrix.org ".to_owned()),
                event_id: Some(" $event:matrix.org ".to_owned()),
            }),
        })
        .expect("notification payload should sanitize");

        assert_eq!(
            payload.actions,
            Some(vec![
                DesktopNotificationAction {
                    id: "agent-approval.approve-once".to_owned(),
                    label: "Approve once".to_owned(),
                },
                DesktopNotificationAction {
                    id: "agent-approval.deny".to_owned(),
                    label: "Deny".to_owned(),
                },
            ])
        );
        assert_eq!(
            payload.action_context,
            Some(DesktopNotificationActionContext {
                kind: "agent-approval".to_owned(),
                room_id: Some("!room:matrix.org".to_owned()),
                event_id: Some("$event:matrix.org".to_owned()),
            })
        );
    }
}
