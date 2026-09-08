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

fn is_time_sensitive_agent_approval(context: Option<&DesktopNotificationActionContext>) -> bool {
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
    static CONFIGURE: std::sync::Once = std::sync::Once::new();
    CONFIGURE.call_once(|| {
        let bundle_identifier = if tauri::is_dev() {
            "com.apple.Terminal"
        } else {
            "com.whylandcreative.synara.desktop"
        };
        if let Err(error) = set_application(bundle_identifier) {
            eprintln!("failed to configure macOS notification application: {error}");
        }
    });
}

/// Delivery receipt for the macOS route/action path.
///
/// `mac_notification_sys::Notification::send()` with `wait_for_click(true)`
/// blocks until the user clicks or the banner is dismissed, and reports a
/// refused delivery as an ordinary auto-dismiss, so it cannot be awaited for
/// a receipt without also waiting for the user. After checking the app's
/// authorization, a new `deliveredNotifications` record confirms Notification
/// Center accepted the post (not that a banner was visible under Focus).
/// Legacy records alone are insufficient: macOS can record a post even while
/// authorization is denied. The identifier set is snapshotted before the send, and the
/// caller is credited once a new identifier carrying this notification's
/// title and body appears, or debited after a bounded wait. Identifiers are
/// claimed once so two in-flight notifications with identical text cannot
/// both be credited by a single delivery. The click wait stays in the
/// background exactly as before.
#[cfg(target_os = "macos")]
mod macos_delivery {
    use std::collections::{HashSet, VecDeque};
    use std::sync::{LazyLock, Mutex};
    use std::time::{Duration, Instant};

    /// Notification Center confirms delivery within a few hundred
    /// milliseconds; the crate's own delivery wait is 2 s.
    pub const RECEIPT_TIMEOUT: Duration = Duration::from_millis(2_500);
    const POLL_INTERVAL: Duration = Duration::from_millis(50);
    const CLAIMED_IDENTIFIERS_MAX: usize = 256;

    /// Read permission on every send: users can disable the app while it is
    /// running. Legacy deliveredNotifications can still record a denied post.
    pub async fn permission_denied() -> Result<bool, String> {
        use block2::RcBlock;
        use objc2_foundation::NSBundle;
        use objc2_user_notifications::{
            UNAuthorizationStatus, UNNotificationSettings, UNUserNotificationCenter,
        };

        // UserNotifications raises an ObjC exception outside an app bundle.
        // Bare development executables use the legacy Terminal identity.
        // The legacy crate can hook bundleIdentifier; bundlePath remains the
        // actual bundle path even if another caller initialized it first.
        if !NSBundle::mainBundle()
            .bundlePath()
            .to_string()
            .ends_with(".app")
        {
            return Ok(false);
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let tx = Mutex::new(Some(tx));
            let completion =
                RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
                    // Apple's completion handler supplies a valid settings object
                    // for the duration of this callback; no reference escapes.
                    let status = unsafe { settings.as_ref() }.authorizationStatus();
                    if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                        let _ = tx.send(status == UNAuthorizationStatus::Denied);
                    }
                });
            UNUserNotificationCenter::currentNotificationCenter()
                .getNotificationSettingsWithCompletionHandler(&completion);
        }
        tokio::time::timeout(RECEIPT_TIMEOUT, rx)
            .await
            .map_err(|_| "macOS notification permission lookup timed out".to_owned())?
            .map_err(|_| "macOS notification permission lookup failed".to_owned())
    }

    /// One entry of `deliveredNotifications`, reduced to what the receipt
    /// compares.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DeliveredRecord {
        pub identifier: String,
        pub title: Option<String>,
        pub body: Option<String>,
    }

    /// Identifiers already credited to a caller (bounded FIFO).
    #[derive(Default)]
    pub struct ClaimedIdentifiers {
        order: VecDeque<String>,
        set: HashSet<String>,
    }

    impl ClaimedIdentifiers {
        fn contains(&self, identifier: &str) -> bool {
            self.set.contains(identifier)
        }

        fn claim(&mut self, identifier: String) {
            if !self.set.insert(identifier.clone()) {
                return;
            }
            self.order.push_back(identifier);
            while self.order.len() > CLAIMED_IDENTIFIERS_MAX {
                if let Some(oldest) = self.order.pop_front() {
                    self.set.remove(&oldest);
                }
            }
        }
    }

    static CLAIMED: LazyLock<Mutex<ClaimedIdentifiers>> =
        LazyLock::new(|| Mutex::new(ClaimedIdentifiers::default()));

    /// Credit exactly one unclaimed, newly delivered record that matches this
    /// notification's title and body. Pure so it can be unit-tested without
    /// Notification Center.
    pub fn select_new_delivery(
        before: &HashSet<String>,
        claimed: &mut ClaimedIdentifiers,
        delivered: &[DeliveredRecord],
        title: &str,
        body: Option<&str>,
    ) -> Option<String> {
        let matched = delivered.iter().find(|record| {
            !before.contains(&record.identifier)
                && !claimed.contains(&record.identifier)
                && record.title.as_deref() == Some(title)
                && record.body.as_deref().unwrap_or_default() == body.unwrap_or_default()
        })?;
        claimed.claim(matched.identifier.clone());
        Some(matched.identifier.clone())
    }

    #[allow(deprecated)]
    fn delivered_records() -> Vec<DeliveredRecord> {
        use objc2_foundation::NSUserNotificationCenter;

        let center = NSUserNotificationCenter::defaultUserNotificationCenter();
        center
            .deliveredNotifications()
            .iter()
            .filter_map(|notification| {
                Some(DeliveredRecord {
                    identifier: notification.identifier()?.to_string(),
                    title: notification.title().map(|value| value.to_string()),
                    body: notification
                        .informativeText()
                        .map(|value| value.to_string()),
                })
            })
            .collect()
    }

    /// Identifiers Notification Center already holds, taken before the send.
    pub fn snapshot_identifiers() -> HashSet<String> {
        delivered_records()
            .into_iter()
            .map(|record| record.identifier)
            .collect()
    }

    /// Wait for Notification Center to record this notification. `Ok(true)`
    /// is a real acceptance; `Ok(false)` means nothing was recorded within
    /// the bound; `Err` surfaces a send failure the blocking task reported.
    pub async fn await_receipt(
        before: HashSet<String>,
        title: String,
        body: Option<String>,
        mut send_failed: tokio::sync::oneshot::Receiver<String>,
    ) -> Result<bool, String> {
        let deadline = Instant::now() + RECEIPT_TIMEOUT;
        loop {
            if let Ok(error) = send_failed.try_recv() {
                return Err(error);
            }
            let credited = {
                let records = delivered_records();
                let mut claimed = CLAIMED
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                select_new_delivery(&before, &mut claimed, &records, &title, body.as_deref())
            };
            if credited.is_some() {
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn record(identifier: &str, title: &str, body: Option<&str>) -> DeliveredRecord {
            DeliveredRecord {
                identifier: identifier.to_owned(),
                title: Some(title.to_owned()),
                body: body.map(str::to_owned),
            }
        }

        #[test]
        fn credits_only_a_new_matching_record_and_only_once() {
            let before: HashSet<String> = ["old".to_owned()].into_iter().collect();
            let mut claimed = ClaimedIdentifiers::default();
            let delivered = vec![
                record("old", "Room", Some("New inbox notification from bob")),
                record(
                    "other",
                    "Other room",
                    Some("New inbox notification from bob"),
                ),
                record("fresh", "Room", Some("New inbox notification from bob")),
            ];

            // An identifier that pre-dates the send is never a receipt for it.
            assert_eq!(
                select_new_delivery(
                    &before,
                    &mut claimed,
                    &delivered[..1],
                    "Room",
                    Some("New inbox notification from bob")
                ),
                None
            );
            // Title and body must both match.
            assert_eq!(
                select_new_delivery(&before, &mut claimed, &delivered, "Room", Some("different")),
                None
            );
            assert_eq!(
                select_new_delivery(
                    &before,
                    &mut claimed,
                    &delivered,
                    "Room",
                    Some("New inbox notification from bob")
                ),
                Some("fresh".to_owned())
            );
            // The same delivery cannot credit a second caller.
            assert_eq!(
                select_new_delivery(
                    &before,
                    &mut claimed,
                    &delivered,
                    "Room",
                    Some("New inbox notification from bob")
                ),
                None
            );
            // A body-less notification only matches a body-less record.
            let bare = vec![record("bare", "Reminder", None)];
            assert_eq!(
                select_new_delivery(&before, &mut claimed, &bare, "Reminder", Some("x")),
                None
            );
            assert_eq!(
                select_new_delivery(&before, &mut claimed, &bare, "Reminder", None),
                Some("bare".to_owned())
            );
        }

        #[test]
        fn claimed_identifiers_are_bounded() {
            let mut claimed = ClaimedIdentifiers::default();
            for index in 0..(CLAIMED_IDENTIFIERS_MAX + 10) {
                claimed.claim(format!("id-{index}"));
            }
            assert_eq!(claimed.order.len(), CLAIMED_IDENTIFIERS_MAX);
            assert_eq!(claimed.set.len(), CLAIMED_IDENTIFIERS_MAX);
            assert!(!claimed.contains("id-0"));
            assert!(claimed.contains(&format!("id-{}", CLAIMED_IDENTIFIERS_MAX + 9)));
        }

        #[test]
        fn absent_body_matches_the_legacy_crates_empty_message() {
            let delivered = [record("empty", "Reminder", Some(""))];
            assert_eq!(
                select_new_delivery(
                    &HashSet::new(),
                    &mut ClaimedIdentifiers::default(),
                    &delivered,
                    "Reminder",
                    None,
                ),
                Some("empty".to_owned())
            );
        }
    }
}

/// Post through `mac-notification-sys` and return Notification Center's
/// acceptance as the receipt. The click/action wait keeps running in the
/// background after the receipt is returned; it never gates the receipt.
#[cfg(target_os = "macos")]
async fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: Option<&str>,
    actions: &[DesktopNotificationAction],
    action_context: Option<&DesktopNotificationActionContext>,
) -> Result<bool, String> {
    use mac_notification_sys::{MainButton, Notification, NotificationResponse, Sound};

    // Check before the legacy crate installs its bundle-identity hook.
    if macos_delivery::permission_denied().await? {
        return Ok(false);
    }
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

    let before = macos_delivery::snapshot_identifiers();
    let (send_failed_tx, send_failed_rx) = tokio::sync::oneshot::channel::<String>();
    let receipt_title = title.clone();
    let receipt_body = body.clone();

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

        let response = match response {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                // Reaches the receipt only while it is still waiting; a late
                // send error after a credited receipt is logged here instead.
                let message = error.to_string();
                if send_failed_tx.send(message.clone()).is_err() {
                    eprintln!("macOS notification send failed after receipt: {message}");
                }
                return;
            }
            Err(error) => {
                let message = format!("notification task failed: {error}");
                if send_failed_tx.send(message.clone()).is_err() {
                    eprintln!("{message}");
                }
                return;
            }
        };

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
    });

    macos_delivery::await_receipt(before, receipt_title, receipt_body, send_failed_rx).await
}

/// Linux: `notify_rust::Notification::show()` is a synchronous D-Bus call;
/// the handle it returns is already the server's acceptance.
#[cfg(target_os = "linux")]
async fn show_notification_with_route_click_handler_receipt<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: Option<&str>,
    actions: &[DesktopNotificationAction],
    action_context: Option<&DesktopNotificationActionContext>,
) -> Result<bool, String> {
    show_notification_with_route_click_handler(app, title, body, route, actions, action_context)
        .map(|()| true)
}

#[cfg(target_os = "macos")]
async fn show_notification_with_route_click_handler_receipt<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: Option<&str>,
    actions: &[DesktopNotificationAction],
    action_context: Option<&DesktopNotificationActionContext>,
) -> Result<bool, String> {
    show_notification_with_route_click_handler(app, title, body, route, actions, action_context)
        .await
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
async fn show_notification_with_route_click_handler_receipt<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    _route: Option<&str>,
    _actions: &[DesktopNotificationAction],
    _action_context: Option<&DesktopNotificationActionContext>,
) -> Result<bool, String> {
    show_notification_without_route_click_handler(app, title, body).map(|()| true)
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

/// Post one desktop notification and return the OS receipt.
///
/// `Ok(true)` means the OS notification server accepted the notification;
/// `Ok(false)` means it did not record one within the platform's bound;
/// `Err` is a post failure. The renderer forwards this verdict unchanged as
/// the `delivered` / `failed` acknowledgement Core's delivery ledger counts,
/// so the command resolves only once the answer is real: on macOS the
/// route/action path awaits Notification Center's acceptance rather than the
/// spawn of the send task, while the click wait continues in the background.
#[tauri::command]
pub async fn desktop_notify<R: Runtime>(
    app: AppHandle<R>,
    notification: DesktopNotificationPayload,
) -> Result<bool, String> {
    let notification = sanitize_notification_payload(notification)?;
    let actions = notification.actions.as_deref().unwrap_or(&[]);

    if cfg!(target_os = "macos") || notification.route.is_some() || !actions.is_empty() {
        return show_notification_with_route_click_handler_receipt(
            &app,
            &notification.title,
            notification.body.as_deref(),
            notification.route.as_deref(),
            actions,
            notification.action_context.as_ref(),
        )
        .await;
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
