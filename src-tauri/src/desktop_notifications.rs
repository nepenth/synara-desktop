use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(target_os = "linux")]
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::desktop::{navigate_main_window, show_main_window};
#[cfg(test)]
use crate::desktop_sanitize::sanitize_route;
use crate::desktop_sanitize::{sanitize_action_text, sanitize_notification_route};

#[cfg(target_os = "macos")]
#[path = "desktop_notifications_macos.rs"]
mod macos_modern;

#[path = "desktop_notification_responses.rs"]
mod responses;

use responses::{
    admit, process_nonce, response_for_action, ActiveSession, Admission, NotificationBinding,
    NotificationResponse, PendingResponses,
};

#[cfg(target_os = "macos")]
pub fn initialize_macos_notifications<R: Runtime>(app: &AppHandle<R>) {
    if macos_delivery::is_bundled() {
        macos_modern::initialize(app);
        // A relaunched process can still withdraw the banners an earlier
        // one posted (sign-out, read elsewhere, decided approvals).
        macos_modern::rebuild_dismiss_index();
    }
}

const DESKTOP_NOTIFICATION_MAX_TITLE_CHARS: usize = 120;
const DESKTOP_NOTIFICATION_MAX_BODY_CHARS: usize = 500;
const DESKTOP_NOTIFICATION_MAX_ACTIONS: usize = 4;
const DESKTOP_NOTIFICATION_MAX_ACTION_ID_CHARS: usize = 96;
const DESKTOP_NOTIFICATION_MAX_ACTION_LABEL_CHARS: usize = 80;
const DESKTOP_NOTIFICATION_MAX_ACTION_CONTEXT_CHARS: usize = 255;
const DESKTOP_NOTIFICATION_MAX_DISMISS_KEYS: usize = 8;
const DESKTOP_NOTIFICATION_MAX_DISMISS_KEY_CHARS: usize = 255;
const DESKTOP_DISMISS_COMMAND_MAX_KEYS: usize = 32;
const MAX_LINUX_NOTIFICATION_HANDLES: usize = 256;
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

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationActionEvent {
    session_generation: Option<u64>,
    action_id: String,
    context: Option<DesktopNotificationActionContext>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopNotificationSoundPolicy {
    Default,
    Silent,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationPayload {
    #[serde(default)]
    pub session_generation: Option<u64>,
    #[serde(default)]
    pub sound: Option<DesktopNotificationSoundPolicy>,
    pub title: String,
    pub body: Option<String>,
    pub route: Option<String>,
    pub actions: Option<Vec<DesktopNotificationAction>>,
    pub action_context: Option<DesktopNotificationActionContext>,
    #[serde(default)]
    pub dismiss_keys: Option<Vec<String>>,
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
    let dismiss_keys = sanitize_dismiss_keys(notification.dismiss_keys);

    Ok(DesktopNotificationPayload {
        session_generation: notification.session_generation,
        sound: notification.sound,
        title,
        body,
        route,
        actions,
        action_context,
        dismiss_keys,
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

pub(crate) fn sanitize_dismiss_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > DESKTOP_NOTIFICATION_MAX_DISMISS_KEY_CHARS
        || trimmed
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
    {
        return None;
    }
    let (prefix, rest) = trimmed.split_once(':')?;
    if prefix == "candidate" {
        let (generation, sequence) = rest.strip_prefix("notif-")?.split_once('-')?;
        return (!generation.is_empty()
            && !sequence.is_empty()
            && generation.bytes().all(|b| b.is_ascii_digit())
            && sequence.bytes().all(|b| b.is_ascii_digit()))
        .then(|| trimmed.to_owned());
    }
    if rest.is_empty() || !matches!(prefix, "room" | "event" | "invite") {
        return None;
    }
    if !trimmed.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '.' | '_' | ':' | '-' | '!' | '$' | '=' | '/' | '+' | '@'
            )
    }) {
        return None;
    }
    Some(trimmed.to_owned())
}

fn sanitize_dismiss_keys(keys: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut sanitized = Vec::new();
    for key in keys.unwrap_or_default() {
        if sanitized.len() >= DESKTOP_NOTIFICATION_MAX_DISMISS_KEYS {
            break;
        }
        let Some(key) = sanitize_dismiss_key(&key) else {
            continue;
        };
        if sanitized.iter().any(|existing| existing == &key) {
            continue;
        }
        sanitized.push(key);
    }
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

#[derive(Debug, Default)]
pub(crate) struct DismissKeyIndex<Id = u32> {
    by_key: HashMap<String, HashSet<Id>>,
    keys_by_id: HashMap<Id, Vec<String>>,
    order: VecDeque<Id>,
}

impl<Id: Clone + Eq + std::hash::Hash> DismissKeyIndex<Id> {
    pub(crate) fn register(&mut self, id: Id, keys: &[String]) -> Vec<Id> {
        let mut evicted = Vec::new();
        self.unregister(id.clone());
        while self.keys_by_id.len() >= MAX_LINUX_NOTIFICATION_HANDLES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.unregister(oldest.clone());
            evicted.push(oldest);
        }
        let mut stored = Vec::new();
        for key in keys {
            if stored.iter().any(|existing| existing == key) {
                continue;
            }
            self.by_key
                .entry(key.clone())
                .or_default()
                .insert(id.clone());
            stored.push(key.clone());
        }
        if !stored.is_empty() {
            self.keys_by_id.insert(id.clone(), stored);
            self.order.push_back(id);
        }
        evicted
    }

    pub(crate) fn unregister(&mut self, id: Id) {
        if let Some(keys) = self.keys_by_id.remove(&id) {
            self.order.retain(|candidate| *candidate != id);
            for key in keys {
                if let Some(ids) = self.by_key.get_mut(&key) {
                    ids.remove(&id);
                    if ids.is_empty() {
                        self.by_key.remove(&key);
                    }
                }
            }
        }
    }

    pub(crate) fn ids_for_keys(&self, keys: &[String]) -> Vec<Id> {
        let mut ids = HashSet::new();
        for key in keys {
            if let Some(set) = self.by_key.get(key) {
                ids.extend(set.iter().cloned());
            }
        }
        ids.into_iter().collect()
    }
}

#[cfg(target_os = "linux")]
fn lock_mutex<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Record a notification failure in the app log as well as stderr, so a
/// report from a user's machine shows why a banner or click did nothing.
fn log_notification_error<R: Runtime>(app: &AppHandle<R>, message: &str) {
    eprintln!("[synara] {message}");
    crate::desktop_logging::append_app_log(app, "notifications", message);
}

fn pending_responses() -> std::sync::MutexGuard<'static, PendingResponses> {
    static PENDING: OnceLock<Mutex<PendingResponses>> = OnceLock::new();
    PENDING
        .get_or_init(|| Mutex::new(PendingResponses::default()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn active_session<R: Runtime>(app: &AppHandle<R>) -> Option<ActiveSession> {
    let auth = app.try_state::<crate::matrix::auth::MatrixAuthState>()?;
    let (generation, account) = auth.active_session_binding().await?;
    Some(ActiveSession {
        generation,
        account,
    })
}

/// Who a banner is posted for: the requested generation plus the account
/// and process that posted it, so a later click can be matched safely.
async fn notification_binding<R: Runtime>(
    app: &AppHandle<R>,
    session_generation: Option<u64>,
) -> NotificationBinding {
    NotificationBinding {
        session_generation,
        account: active_session(app).await.map(|active| active.account),
        process: Some(process_nonce()),
    }
}

/// Deliver one response under its session's generation gate. A route shows
/// the window; an action reaches the renderer's listener.
async fn deliver_notification_response<R: Runtime>(
    app: &AppHandle<R>,
    generation: Option<u64>,
    response: NotificationResponse,
) -> Result<(), String> {
    let deliver = || async {
        match &response {
            NotificationResponse::Open { route } => {
                navigate_main_window(app, route).map_err(|error| error.to_string())
            }
            NotificationResponse::Action { action_id, context } => app
                .emit(
                    DESKTOP_NOTIFICATION_ACTION_EVENT,
                    DesktopNotificationActionEvent {
                        session_generation: generation,
                        action_id: action_id.clone(),
                        context: context.clone(),
                    },
                )
                .map_err(|error| error.to_string()),
        }
    };
    let Some(generation) = generation else {
        return deliver().await;
    };
    let auth = app.state::<crate::matrix::auth::MatrixAuthState>();
    auth.with_session_generation(generation, deliver)
        .await
        .unwrap_or_else(|| Err("the session changed before the response arrived".to_owned()))
}

/// Act on a banner click or action button: deliver it now, keep it until
/// the renderer is ready (cold launch, before restore, after re-login), or
/// refuse it. Whatever happens, a click that cannot be delivered still shows
/// the window, so it never appears to do nothing.
fn handle_notification_response<R: Runtime>(
    app: &AppHandle<R>,
    binding: NotificationBinding,
    critical: bool,
    response: NotificationResponse,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let active = active_session(&app).await;
        let ready = pending_responses().renderer_ready();
        let result = match admit(&binding, critical, active.as_ref(), ready, process_nonce()) {
            Admission::Deliver { generation } => {
                deliver_notification_response(&app, generation, response).await
            }
            Admission::Queue => {
                pending_responses().push(binding, critical, response, Instant::now());
                show_main_window(&app).map_err(|error| error.to_string())
            }
            Admission::Reject => {
                Err("the notification belongs to another session or account".to_owned())
            }
        };
        if let Err(error) = result {
            log_notification_error(
                &app,
                &format!("notification response not delivered: {error}"),
            );
            if let Err(error) = show_main_window(&app) {
                log_notification_error(&app, &format!("failed to show Synara: {error}"));
            }
        }
    });
}

fn is_time_sensitive_agent_approval(context: Option<&DesktopNotificationActionContext>) -> bool {
    context.is_some_and(|context| context.kind == "agent-approval")
}

/// Approval banners: never delivered without a matching session or account.
fn is_critical_notification(
    context: Option<&DesktopNotificationActionContext>,
    actions: &[DesktopNotificationAction],
) -> bool {
    is_time_sensitive_agent_approval(context)
        || actions
            .iter()
            .any(|action| action.id.starts_with("agent-approval."))
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
fn linux_notification_handles() -> &'static Mutex<HashMap<u32, Arc<notify_rust::NotificationHandle>>>
{
    static HANDLES: OnceLock<Mutex<HashMap<u32, Arc<notify_rust::NotificationHandle>>>> =
        OnceLock::new();
    HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Waiter tasks per notification id so a dismiss or eviction can abort a
/// `wait_for_action_async` that would otherwise hang when the daemon's
/// `NotificationClosed` signal was missed (closed before the match rule was
/// installed, or the daemon restarted).
#[cfg(target_os = "linux")]
fn linux_notification_waiters() -> &'static Mutex<HashMap<u32, tauri::async_runtime::JoinHandle<()>>>
{
    static WAITERS: OnceLock<Mutex<HashMap<u32, tauri::async_runtime::JoinHandle<()>>>> =
        OnceLock::new();
    WAITERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Upper bound on how long a Linux waiter task may outlive its notification.
/// The longest notification timeout we request is 300 s (agent approvals).
#[cfg(target_os = "linux")]
const LINUX_NOTIFICATION_WAIT_MAX: std::time::Duration = std::time::Duration::from_secs(15 * 60);

#[cfg(target_os = "linux")]
fn abort_linux_waiter(id: u32) {
    if let Some(waiter) = lock_mutex(linux_notification_waiters()).remove(&id) {
        waiter.abort();
    }
}

#[cfg(target_os = "linux")]
fn linux_dismiss_index() -> &'static Mutex<DismissKeyIndex> {
    static INDEX: OnceLock<Mutex<DismissKeyIndex>> = OnceLock::new();
    INDEX.get_or_init(|| Mutex::new(DismissKeyIndex::default()))
}

#[cfg(target_os = "linux")]
fn register_linux_notification(
    id: u32,
    keys: &[String],
    handle: Arc<notify_rust::NotificationHandle>,
) {
    if !keys.is_empty() {
        let _ = lock_mutex(linux_dismiss_index()).register(id, keys);
    }
    let evicted = {
        let mut handles = lock_mutex(linux_notification_handles());
        let mut evicted = Vec::new();
        while handles.len() >= MAX_LINUX_NOTIFICATION_HANDLES {
            let Some(old_id) = handles.keys().min().copied() else {
                break;
            };
            if let Some(old_handle) = handles.remove(&old_id) {
                evicted.push((old_id, old_handle));
            }
        }
        handles.insert(id, handle);
        evicted
    };
    if !evicted.is_empty() {
        let mut index = lock_mutex(linux_dismiss_index());
        for (old_id, _) in &evicted {
            index.unregister(*old_id);
        }
    }
    for (old_id, handle) in evicted {
        tauri::async_runtime::spawn(async move {
            handle.close_async().await;
            abort_linux_waiter(old_id);
        });
    }
}

#[cfg(target_os = "linux")]
fn unregister_linux_notification(id: u32) {
    lock_mutex(linux_dismiss_index()).unregister(id);
    lock_mutex(linux_notification_handles()).remove(&id);
    // Called from the waiter itself on completion; it is finished, so only
    // drop the bookkeeping entry rather than aborting.
    lock_mutex(linux_notification_waiters()).remove(&id);
}

#[cfg(target_os = "linux")]
async fn dismiss_linux_notifications(keys: &[String]) {
    let ids = lock_mutex(linux_dismiss_index()).ids_for_keys(keys);
    let live: Vec<(u32, Arc<notify_rust::NotificationHandle>)> = {
        let mut handles = lock_mutex(linux_notification_handles());
        ids.iter()
            .filter_map(|id| handles.remove(id).map(|handle| (*id, handle)))
            .collect()
    };
    {
        let mut index = lock_mutex(linux_dismiss_index());
        for id in &ids {
            index.unregister(*id);
        }
    }
    for (id, handle) in live {
        handle.close_async().await;
        abort_linux_waiter(id);
    }
}

/// Close every banner this process still tracks (sign-out).
#[cfg(target_os = "linux")]
async fn dismiss_all_linux_notifications() {
    let live: Vec<(u32, Arc<notify_rust::NotificationHandle>)> =
        lock_mutex(linux_notification_handles()).drain().collect();
    *lock_mutex(linux_dismiss_index()) = DismissKeyIndex::default();
    for (id, handle) in live {
        handle.close_async().await;
        abort_linux_waiter(id);
    }
}

/// The server's advertised `actions` capability, read once per process.
/// An unreachable server is treated as supporting actions so the existing
/// delivery error path, not this hint, reports the failure.
#[cfg(target_os = "linux")]
fn linux_server_supports_actions() -> bool {
    static SUPPORTS_ACTIONS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SUPPORTS_ACTIONS.get_or_init(|| {
        notify_rust::get_capabilities()
            .map(|capabilities| {
                capabilities
                    .iter()
                    .any(|capability| capability == "actions")
            })
            .unwrap_or(true)
    })
}

/// The freedesktop sound for a notification that may sound. Daemons that
/// honour `sound-name` (GNOME Shell, Plasma) play it under the desktop's
/// own Do Not Disturb rules.
#[cfg(any(target_os = "linux", test))]
fn linux_notification_sound_name(
    sound: Option<DesktopNotificationSoundPolicy>,
) -> Option<&'static str> {
    match sound {
        Some(DesktopNotificationSoundPolicy::Silent) => None,
        Some(DesktopNotificationSoundPolicy::Default) | None => Some("message-new-instant"),
    }
}

#[cfg(target_os = "linux")]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
    binding: &NotificationBinding,
) -> Result<(), String> {
    let title = notification.title.as_str();
    let body = notification.body.as_deref();
    let route = notification.route.as_deref();
    let actions = notification.actions.as_deref().unwrap_or(&[]);
    let action_context = notification.action_context.as_ref();
    let critical = is_critical_notification(action_context, actions);
    let dismiss_keys = notification.dismiss_keys.as_deref().unwrap_or(&[]);

    use notify_rust::{Notification, Urgency};
    let notification_payload_sound = notification.sound;

    // Some notification servers (for example mako, or dunst without a
    // context menu) do not render action buttons. Say where the decision can
    // be made instead of showing an approval that silently cannot be acted on.
    let body = if !actions.is_empty() && !linux_server_supports_actions() {
        Some(match body {
            Some(body) => format!("{body}\n\nOpen Synara to approve or deny."),
            None => "Open Synara to approve or deny.".to_owned(),
        })
    } else {
        body.map(str::to_owned)
    };

    let mut notification = Notification::new();
    notification.summary(title);
    if let Some(body) = body.as_deref() {
        notification.body(body);
    }
    notification.auto_icon();
    match linux_notification_sound_name(notification_payload_sound) {
        Some(sound) => {
            notification.sound_name(sound);
        }
        None => {
            notification.hint(notify_rust::Hint::SuppressSound(true));
        }
    }
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

    let handle = Arc::new(notification.show().map_err(|error| error.to_string())?);
    let id = handle.id();
    register_linux_notification(id, dismiss_keys, handle.clone());
    let app = app.clone();
    let route = route.map(str::to_owned);
    let action_context = action_context.cloned();
    let binding = binding.clone();
    let allowed_action_ids = actions
        .iter()
        .map(|action| action.id.clone())
        .collect::<Vec<_>>();

    let waiter_app = app.clone();
    let waiter = tauri::async_runtime::spawn(async move {
        let wait = handle.wait_for_action_async(move |response| match response {
            notify_rust::NotificationResponse::Default => {
                let Some(route) = route.clone() else {
                    return;
                };
                handle_notification_response(
                    &app,
                    binding.clone(),
                    critical,
                    NotificationResponse::Open { route },
                );
            }
            notify_rust::NotificationResponse::Action(action) => {
                if allowed_action_ids
                    .iter()
                    .any(|candidate| candidate == action)
                {
                    handle_notification_response(
                        &app,
                        binding.clone(),
                        critical,
                        response_for_action(action, route.as_deref(), action_context.clone()),
                    );
                }
            }
            notify_rust::NotificationResponse::Closed(_)
            | notify_rust::NotificationResponse::Reply(_) => {}
        });
        if tokio::time::timeout(LINUX_NOTIFICATION_WAIT_MAX, wait)
            .await
            .is_err()
        {
            log_notification_error(
                &waiter_app,
                &format!("notification {id} waiter timed out without a close signal"),
            );
        }
        unregister_linux_notification(id);
    });
    lock_mutex(linux_notification_waiters()).insert(id, waiter);

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
            // Development only (unbundled); there is no AppHandle here.
            eprintln!("failed to configure macOS notification application: {error}");
        }
    });
}

/// Shared permission lookup and legacy receipts for unbundled development.
/// Bundled applications use UserNotifications exclusively (macos_modern).
/// The unbundled legacy path credits a new delivered record within a bounded
/// wait, independently of its background click wait. Identifiers are claimed
/// once so identical concurrent notifications cannot share one receipt.
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

    /// Detect a real app bundle without relying on a hooked bundle identifier.
    pub fn is_bundled() -> bool {
        objc2_foundation::NSBundle::mainBundle()
            .bundlePath()
            .to_string()
            .ends_with(".app")
    }

    pub async fn authorization_status(
    ) -> Result<objc2_user_notifications::UNAuthorizationStatus, String> {
        notification_settings().await.map(|settings| settings.0)
    }

    pub async fn notification_settings() -> Result<
        (
            objc2_user_notifications::UNAuthorizationStatus,
            objc2_user_notifications::UNNotificationSetting,
        ),
        String,
    > {
        use block2::RcBlock;
        use objc2_user_notifications::{
            UNAuthorizationStatus, UNNotificationSetting, UNNotificationSettings,
            UNUserNotificationCenter,
        };

        // UserNotifications raises an ObjC exception outside an app bundle.
        // Bare development executables use the legacy Terminal identity.
        // The legacy crate can hook bundleIdentifier; bundlePath remains the
        // actual bundle path even if another caller initialized it first.
        if !is_bundled() {
            return Ok((
                UNAuthorizationStatus::Authorized,
                UNNotificationSetting::NotSupported,
            ));
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let tx = Mutex::new(Some(tx));
            let completion =
                RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
                    // Apple's completion handler supplies a valid settings object
                    // for the duration of this callback; no reference escapes.
                    let settings = unsafe { settings.as_ref() };
                    let status = (
                        settings.authorizationStatus(),
                        settings.criticalAlertSetting(),
                    );
                    if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                        let _ = tx.send(status);
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
        use objc2::{msg_send, rc::Retained};
        use objc2_foundation::{NSArray, NSUserNotification, NSUserNotificationCenter};

        let center = NSUserNotificationCenter::defaultUserNotificationCenter();
        // macOS can return nil before this center has delivered anything,
        // despite the non-null annotation in Foundation's generated binding.
        // Preserve that actual ABI contract instead of panicking and leaving
        // the caller's delivery receipt unresolved.
        let notifications: Option<Retained<NSArray<NSUserNotification>>> =
            unsafe { msg_send![&*center, deliveredNotifications] };
        let Some(notifications) = notifications else {
            return Vec::new();
        };
        notifications
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
    notification: &DesktopNotificationPayload,
    binding: &NotificationBinding,
) -> Result<bool, String> {
    let title = notification.title.as_str();
    let body = notification.body.as_deref();
    let route = notification.route.as_deref();
    let actions = notification.actions.as_deref().unwrap_or(&[]);
    let action_context = notification.action_context.as_ref();
    let critical = is_critical_notification(action_context, actions);

    use mac_notification_sys::{MainButton, Notification, Sound};

    if macos_delivery::is_bundled() {
        return macos_modern::show(app, notification, binding).await;
    }
    if notification.session_generation.is_some() {
        // A legacy development sender has no callback that can keep the auth
        // gate through actual acceptance. Bound candidates fail closed there.
        return Ok(false);
    }
    // Bare development executables retain their legacy Terminal identity.
    // Never mix that center with UserNotifications in a bundled application.
    configure_macos_notification_application();

    let title = title.to_owned();
    let body = body.map(str::to_owned);
    let app = app.clone();
    let route = route.map(str::to_owned);
    let action_context = action_context.cloned();
    let time_sensitive = is_time_sensitive_agent_approval(action_context.as_ref());
    let silent = notification.sound == Some(DesktopNotificationSoundPolicy::Silent);
    let binding = binding.clone();
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
            }
            if !silent {
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
                    log_notification_error(
                        &app,
                        &format!("macOS notification send failed after receipt: {message}"),
                    );
                }
                return;
            }
            Err(error) => {
                let message = format!("notification task failed: {error}");
                if send_failed_tx.send(message.clone()).is_err() {
                    log_notification_error(&app, &message);
                }
                return;
            }
        };

        match response {
            mac_notification_sys::NotificationResponse::Click => {
                let Some(route) = route.clone() else {
                    return;
                };
                handle_notification_response(
                    &app,
                    binding,
                    critical,
                    NotificationResponse::Open { route },
                );
            }
            mac_notification_sys::NotificationResponse::ActionButton(label) => {
                let Some((_, action_id)) = action_ids_by_label
                    .iter()
                    .find(|(candidate_label, _)| candidate_label == &label)
                else {
                    return;
                };
                handle_notification_response(
                    &app,
                    binding,
                    critical,
                    response_for_action(action_id, route.as_deref(), action_context.clone()),
                );
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
    notification: &DesktopNotificationPayload,
    binding: &NotificationBinding,
) -> Result<bool, String> {
    show_notification_with_route_click_handler(app, notification, binding).map(|()| true)
}

#[cfg(target_os = "macos")]
async fn show_notification_with_route_click_handler_receipt<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
    binding: &NotificationBinding,
) -> Result<bool, String> {
    show_notification_with_route_click_handler(app, notification, binding).await
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
async fn show_notification_with_route_click_handler_receipt<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
    _binding: &NotificationBinding,
) -> Result<bool, String> {
    show_notification_without_route_click_handler(
        app,
        &notification.title,
        notification.body.as_deref(),
    )
    .map(|()| true)
}

/// What the OS allows: UserNotifications on a bundled macOS build (where
/// `default` means not asked yet), otherwise the notification plugin.
pub(crate) async fn notification_permission<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    if macos_delivery::is_bundled() {
        return macos_modern::permission().await;
    }
    app.notification()
        .permission_state()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn desktop_get_notification_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<String, String> {
    notification_permission(&app).await
}

/// Ask the OS for permission. Only the user's own request (Settings, or the
/// renderer's one-time prompt) reaches this; posting never asks.
#[tauri::command]
pub async fn desktop_request_notification_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    if macos_delivery::is_bundled() {
        return macos_modern::request_permission().await;
    }
    app.notification()
        .request_permission()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

/// Post without holding the session lock. A bound post checks that its
/// generation is installed, posts, and checks again: if a logout or account
/// switch landed meanwhile, the banner is withdrawn and reported undelivered.
/// The OS post (and any wait for its receipt) therefore never stalls media,
/// sync status or sign-out behind the session lock.
async fn post_for_generation<Current, CurrentFut, Post, PostFut, Withdraw, WithdrawFut>(
    expected: Option<u64>,
    current: Current,
    post: Post,
    withdraw: Withdraw,
) -> Result<bool, String>
where
    Current: Fn() -> CurrentFut,
    CurrentFut: std::future::Future<Output = Option<u64>>,
    Post: FnOnce() -> PostFut,
    PostFut: std::future::Future<Output = Result<bool, String>>,
    Withdraw: FnOnce() -> WithdrawFut,
    WithdrawFut: std::future::Future<Output = ()>,
{
    let Some(expected) = expected else {
        return post().await;
    };
    if current().await != Some(expected) {
        return Ok(false);
    }
    let accepted = post().await?;
    if accepted && current().await != Some(expected) {
        withdraw().await;
        return Ok(false);
    }
    Ok(accepted)
}

/// Post one desktop notification and return the OS receipt.
///
/// `Ok(true)` means the OS notification server accepted the notification;
/// `Ok(false)` means permission or submission was refused, the bound session
/// ended, or no legacy record appeared within the platform's bound;
/// `Err` is a post failure. The renderer forwards this verdict unchanged as
/// the `delivered` / `failed` acknowledgement Core's delivery ledger counts,
/// so the command resolves only once the answer is real: on macOS the
/// route/action path awaits Notification Center's acceptance rather than the
/// spawn of the send task. Later clicks never gate the delivery receipt.
#[tauri::command]
pub async fn desktop_notify<R: Runtime>(
    app: AppHandle<R>,
    notification: DesktopNotificationPayload,
) -> Result<bool, String> {
    let notification = sanitize_notification_payload(notification)?;
    // The acceptance task owns the AppHandle. A renderer that stops waiting
    // must not skip the re-check that withdraws a banner of an ended session.
    tauri::async_runtime::spawn(async move {
        let binding = notification_binding(&app, notification.session_generation).await;
        let keys = notification.dismiss_keys.clone().unwrap_or_default();
        post_for_generation(
            notification.session_generation,
            || async { active_session(&app).await.map(|active| active.generation) },
            || post_sanitized_notification(&app, &notification, &binding),
            || dismiss_notifications_now(&keys),
        )
        .await
    })
    .await
    .map_err(|_| "Native notification acceptance task failed".to_owned())?
}

async fn post_sanitized_notification<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
    binding: &NotificationBinding,
) -> Result<bool, String> {
    // macOS and Linux always post through their own clients, which carry the
    // click binding, dismiss keys and sound policy.
    if cfg!(any(target_os = "macos", target_os = "linux"))
        || notification.route.is_some()
        || notification
            .actions
            .as_ref()
            .is_some_and(|actions| !actions.is_empty())
        || notification
            .dismiss_keys
            .as_ref()
            .is_some_and(|keys| !keys.is_empty())
        || notification.sound == Some(DesktopNotificationSoundPolicy::Silent)
    {
        return show_notification_with_route_click_handler_receipt(app, notification, binding)
            .await;
    }
    show_notification_without_route_click_handler(
        app,
        &notification.title,
        notification.body.as_deref(),
    )?;
    Ok(true)
}

async fn dismiss_notifications_now(keys: &[String]) {
    if keys.is_empty() {
        return;
    }
    #[cfg(target_os = "linux")]
    dismiss_linux_notifications(keys).await;
    #[cfg(target_os = "macos")]
    if macos_delivery::is_bundled() {
        macos_modern::dismiss(keys);
    }
}

async fn dismiss_all_notifications() {
    #[cfg(target_os = "linux")]
    dismiss_all_linux_notifications().await;
    #[cfg(target_os = "macos")]
    if macos_delivery::is_bundled() {
        macos_modern::dismiss_all();
    }
}

/// Close delivered Linux notifications that were tagged with `dismissKeys`.
///
/// Bundled macOS cancels the mapped pending and delivered request identifiers.
/// Legacy bare development executables and unsupported platforms have no mapping.
#[tauri::command]
pub async fn desktop_dismiss_notifications(keys: Vec<String>) -> Result<(), String> {
    let keys: Vec<String> = keys
        .into_iter()
        .take(DESKTOP_DISMISS_COMMAND_MAX_KEYS)
        .filter_map(|key| sanitize_dismiss_key(&key))
        .collect();
    dismiss_notifications_now(&keys).await;
    Ok(())
}

/// Withdraw every banner this app has posted: all of this app's delivered
/// and pending requests on macOS (including an earlier launch's), every
/// tracked handle on Linux.
#[tauri::command]
pub async fn desktop_dismiss_all_notifications() -> Result<(), String> {
    dismiss_all_notifications().await;
    Ok(())
}

/// The renderer is listening for `session_generation`: hand it the clicks
/// and actions that arrived before it was (cold launch, before restore,
/// after re-login). Routes are opened here; actions are returned for the
/// renderer's action handler. Nothing is returned for another generation.
#[tauri::command]
pub async fn desktop_take_notification_responses<R: Runtime>(
    app: AppHandle<R>,
    session_generation: u64,
) -> Result<Vec<DesktopNotificationActionEvent>, String> {
    let Some(active) = active_session(&app).await else {
        return Ok(Vec::new());
    };
    if active.generation != session_generation {
        return Ok(Vec::new());
    }
    let drained = pending_responses().drain_for(&active, process_nonce(), Instant::now());
    let mut actions = Vec::new();
    for (response, generation) in drained {
        match response {
            NotificationResponse::Action { action_id, context } => {
                actions.push(DesktopNotificationActionEvent {
                    session_generation: generation,
                    action_id,
                    context,
                });
            }
            open @ NotificationResponse::Open { .. } => {
                if let Err(error) = deliver_notification_response(&app, generation, open).await {
                    log_notification_error(
                        &app,
                        &format!("queued notification click not opened: {error}"),
                    );
                }
            }
        }
    }
    Ok(actions)
}

/// Sign-out (voluntary, forced or expired): no banner, count or queued click
/// of the old session may outlive it, even when the renderer that would
/// clean up has already gone.
pub(crate) async fn clear_signed_out_surfaces<R: Runtime>(app: &AppHandle<R>) {
    pending_responses().reset();
    dismiss_all_notifications().await;
    if let Err(error) = crate::desktop_tray::set_badge_count(app, None) {
        log_notification_error(app, &format!("failed to clear the badge: {error}"));
    }
    if let Err(error) = crate::desktop_tray::reset_tray_counts(app) {
        log_notification_error(app, &format!("failed to reset the tray: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_notification_payload_rejects_empty_title() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            session_generation: None,
            sound: None,
            title: "  ".to_owned(),
            body: Some("Body".to_owned()),
            route: None,
            actions: None,
            action_context: None,
            dismiss_keys: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_notification_payload_truncates_body() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            session_generation: None,
            sound: None,
            title: "Reminder".to_owned(),
            body: Some("a".repeat(DESKTOP_NOTIFICATION_MAX_BODY_CHARS + 10)),
            route: Some("/inbox/".to_owned()),
            actions: None,
            action_context: None,
            dismiss_keys: None,
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
            session_generation: None,
            sound: None,
            title: "Later".to_owned(),
            body: Some("Reminder".to_owned()),
            route: Some("/inbox/later/".to_owned()),
            actions: None,
            action_context: None,
            dismiss_keys: None,
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
            session_generation: None,
            sound: None,
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("/inbox/notifications/".to_owned()),
            actions: None,
            action_context: None,
            dismiss_keys: None,
        })
        .expect("notification payload should pass");
        assert_eq!(payload.route, Some("/inbox/notifications/".to_string()));
    }

    #[test]
    fn sanitize_notification_payload_rejects_unsafe_route() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            session_generation: None,
            sound: None,
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("https://evil.example.com".to_owned()),
            actions: None,
            action_context: None,
            dismiss_keys: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_notification_payload_sanitizes_actions_and_context() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            session_generation: None,
            sound: None,
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
            dismiss_keys: Some(vec![
                " room:!room:matrix.org ".to_owned(),
                "event:$event:matrix.org".to_owned(),
                "bad key".to_owned(),
            ]),
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
        assert_eq!(
            payload.dismiss_keys,
            Some(vec![
                "room:!room:matrix.org".to_owned(),
                "event:$event:matrix.org".to_owned(),
            ])
        );
    }

    #[test]
    fn native_payload_retains_closed_sound_policy_and_optional_generation_binding() {
        let notification: DesktopNotificationPayload = serde_json::from_value(serde_json::json!({
            "title": "Message", "sessionGeneration": 7, "sound": "silent",
            "dismissKeys": ["candidate:notif-7-12"]
        }))
        .unwrap();
        let sanitized = sanitize_notification_payload(notification).unwrap();
        assert_eq!(sanitized.session_generation, Some(7));
        assert_eq!(
            sanitized.sound,
            Some(DesktopNotificationSoundPolicy::Silent)
        );
        assert_eq!(
            sanitized.dismiss_keys,
            Some(vec!["candidate:notif-7-12".to_owned()])
        );
        for sound in ["muted", "other", "Silent"] {
            assert!(serde_json::from_value::<DesktopNotificationPayload>(
                serde_json::json!({"title": "Message", "sound": sound})
            )
            .is_err());
        }
        let unbound: DesktopNotificationPayload =
            serde_json::from_value(serde_json::json!({"title": "System notice"})).unwrap();
        assert!(unbound.session_generation.is_none());
        assert!(unbound.sound.is_none());
    }

    #[test]
    fn sanitize_dismiss_key_allows_matrix_ids_and_rejects_unsafe_values() {
        assert_eq!(
            sanitize_dismiss_key("room:!room:example.org"),
            Some("room:!room:example.org".to_owned())
        );
        assert_eq!(
            sanitize_dismiss_key("event:$event:example.org"),
            Some("event:$event:example.org".to_owned())
        );
        assert_eq!(
            sanitize_dismiss_key("candidate:notif-7-12"),
            Some("candidate:notif-7-12".to_owned())
        );
        for key in [
            "candidate:notif-7",
            "candidate:notif-7-12-extra",
            "candidate:notif-x-12",
            "candidate:notif-7-1\n2",
            "candidate:other-7-12",
        ] {
            assert_eq!(sanitize_dismiss_key(key), None, "{key:?}");
        }
        assert_eq!(
            sanitize_dismiss_key("invite:!room:example.org"),
            Some("invite:!room:example.org".to_owned())
        );
        assert_eq!(sanitize_dismiss_key("invite:"), None);
        assert_eq!(sanitize_dismiss_key("other:abc"), None);
        assert_eq!(sanitize_dismiss_key("room:"), None);
        assert_eq!(sanitize_dismiss_key("room:!room example"), None);
        assert_eq!(sanitize_dismiss_key(&"x".repeat(300)), None);
        assert_eq!(
            sanitize_dismiss_key("event:$abc+/=_-:example.org"),
            Some("event:$abc+/=_-:example.org".to_owned())
        );
        assert_eq!(sanitize_dismiss_key("room:!ünicode:example.org"), None);
        assert_eq!(sanitize_dismiss_key("room:!room:example.org\u{202E}"), None);
        assert_ne!(
            sanitize_dismiss_key("room:event:$x"),
            sanitize_dismiss_key("event:$x")
        );
    }

    #[test]
    fn dismiss_key_index_removes_a_handle_from_every_key() {
        let mut index = DismissKeyIndex::default();
        index.register(
            1,
            &["room:!a:example.org".to_owned(), "event:$one".to_owned()],
        );
        index.register(2, &["room:!a:example.org".to_owned()]);
        let mut ids = index.ids_for_keys(&["event:$one".to_owned()]);
        ids.sort_unstable();
        assert_eq!(ids, vec![1]);
        index.unregister(1);
        let mut remaining = index.ids_for_keys(&["room:!a:example.org".to_owned()]);
        remaining.sort_unstable();
        assert_eq!(remaining, vec![2]);
        assert!(index.ids_for_keys(&["event:$one".to_owned()]).is_empty());
    }

    #[test]
    fn dismiss_key_index_is_bounded() {
        let mut index = DismissKeyIndex::default();
        for id in 0..=MAX_LINUX_NOTIFICATION_HANDLES as u32 {
            index.register(id, &[format!("room:!{id}:example.org")]);
        }
        assert_eq!(index.keys_by_id.len(), MAX_LINUX_NOTIFICATION_HANDLES);
        assert!(index
            .ids_for_keys(&["room:!0:example.org".to_owned()])
            .is_empty());
        assert_eq!(
            index.ids_for_keys(&[format!(
                "room:!{}:example.org",
                MAX_LINUX_NOTIFICATION_HANDLES
            )]),
            vec![MAX_LINUX_NOTIFICATION_HANDLES as u32]
        );
    }

    #[tokio::test]
    async fn a_bound_post_never_holds_the_session_lock_and_withdraws_after_logout() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let session = tokio::sync::Mutex::new(Some(7_u64));
        let current = || async { *session.lock().await };
        let withdrawn = AtomicUsize::new(0);
        let withdraw = || async {
            withdrawn.fetch_add(1, Ordering::SeqCst);
        };

        // The OS post (which can wait for the user or Notification Center)
        // runs with the session lock free, so media and sign-out never stall.
        let posted = post_for_generation(
            Some(7),
            current,
            || async {
                assert!(
                    session.try_lock().is_ok(),
                    "post ran under the session lock"
                );
                Ok(true)
            },
            withdraw,
        )
        .await;
        assert_eq!(posted, Ok(true));
        assert_eq!(withdrawn.load(Ordering::SeqCst), 0);

        // Another generation: nothing is posted.
        let posted = post_for_generation(
            Some(6),
            current,
            || async { panic!("a stale generation must not post") },
            withdraw,
        )
        .await;
        assert_eq!(posted, Ok(false));

        // A sign-out while the OS accepted the banner withdraws it.
        let posted = post_for_generation(
            Some(7),
            current,
            || async {
                *session.lock().await = None;
                Ok(true)
            },
            withdraw,
        )
        .await;
        assert_eq!(posted, Ok(false));
        assert_eq!(withdrawn.load(Ordering::SeqCst), 1);

        // Unbound system notices post whatever the session.
        let posted = post_for_generation(None, current, || async { Ok(true) }, withdraw).await;
        assert_eq!(posted, Ok(true));

        let source = include_str!("desktop_notifications.rs");
        let notify = source
            .split("pub async fn desktop_notify<")
            .nth(1)
            .and_then(|rest| rest.split("\nasync fn post_sanitized_notification").next())
            .expect("desktop_notify body");
        assert!(notify.contains("post_for_generation("));
        assert!(!notify.contains("with_session_generation"));
    }

    #[test]
    fn notification_failures_reach_the_app_log_and_clicks_always_show_the_window() {
        let source = include_str!("desktop_notifications.rs")
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("production source");
        assert!(!source.contains("let _ = auth"));
        assert!(!source.contains("eprintln!(\"failed to navigate"));
        assert!(!source.contains("eprintln!(\"failed to emit"));
        let handler = source
            .split("fn handle_notification_response<")
            .nth(1)
            .and_then(|rest| rest.split("\nfn is_time_sensitive_agent_approval").next())
            .expect("response handler");
        // Waiting for the session, refused, or failed: the window comes up.
        assert!(handler.contains(
            "pending_responses().push(binding, critical, response, Instant::now());\n                show_main_window(&app)"
        ));
        assert!(handler.contains("log_notification_error(&app"));
        assert!(handler.matches("show_main_window(&app)").count() >= 2);
        let macos = include_str!("desktop_notifications_macos.rs");
        assert!(!macos.contains("let _ = navigate_notification_route"));
        assert!(!macos.contains("let _ = emit_notification_action"));
    }

    #[test]
    fn linux_messages_ask_the_daemon_for_a_sound_unless_silent() {
        assert_eq!(
            linux_notification_sound_name(Some(DesktopNotificationSoundPolicy::Default)),
            Some("message-new-instant")
        );
        assert_eq!(
            linux_notification_sound_name(None),
            Some("message-new-instant")
        );
        assert_eq!(
            linux_notification_sound_name(Some(DesktopNotificationSoundPolicy::Silent)),
            None
        );
    }

    #[test]
    fn queued_actions_reach_the_renderer_in_the_listener_shape() {
        let event = DesktopNotificationActionEvent {
            session_generation: Some(3),
            action_id: "agent-approval.deny".to_owned(),
            context: Some(DesktopNotificationActionContext {
                kind: "agent-approval".to_owned(),
                room_id: Some("!r:x".to_owned()),
                event_id: Some("$e".to_owned()),
            }),
        };
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            serde_json::json!({
                "sessionGeneration": 3,
                "actionId": "agent-approval.deny",
                "context": {"kind": "agent-approval", "roomId": "!r:x", "eventId": "$e"}
            })
        );
    }

    #[test]
    fn sign_out_withdraws_every_banner_and_forgets_queued_clicks() {
        let source = include_str!("desktop_notifications.rs");
        let clear = source
            .split("pub(crate) async fn clear_signed_out_surfaces<")
            .nth(1)
            .and_then(|rest| rest.split("\n}\n").next())
            .expect("clear_signed_out_surfaces body");
        assert!(clear.contains("pending_responses().reset();"));
        assert!(clear.contains("dismiss_all_notifications().await;"));
        assert!(clear.contains("set_badge_count(app, None)"));
        assert!(clear.contains("reset_tray_counts(app)"));
        let all = source
            .split("async fn dismiss_all_notifications()")
            .nth(1)
            .and_then(|rest| rest.split("\n}\n").next())
            .expect("dismiss_all_notifications body");
        assert!(all.contains("dismiss_all_linux_notifications().await"));
        assert!(all.contains("macos_modern::dismiss_all()"));
        let macos = include_str!("desktop_notifications_macos.rs");
        assert!(macos.contains("center.removeAllPendingNotificationRequests();"));
        assert!(macos.contains("center.removeAllDeliveredNotifications();"));
    }
}
