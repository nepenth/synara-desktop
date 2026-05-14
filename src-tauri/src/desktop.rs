use serde::Serialize;
use std::collections::HashMap;
use std::env;
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use url::Url;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use std::fs;

use crate::build_info;

pub const MAIN_WINDOW_LABEL: &str = "main";

const MENU_SHOW: &str = "desktop.show";
const MENU_LATER: &str = "desktop.later";
const MENU_NOTIFICATIONS: &str = "desktop.notifications";
const MENU_UNREAD_SUMMARY: &str = "desktop.unread-summary";
const MENU_DESKTOP_INTEGRATION: &str = "desktop.integration";
const MENU_DND_TOGGLE: &str = "desktop.dnd";
const MENU_BUILD_INFO: &str = "desktop.build-info";
const MENU_QUIT: &str = "desktop.quit";

const ROUTE_HOME: &str = "/";
const ROUTE_LATER: &str = "/inbox/later/";
const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";
const ROUTE_SETTINGS: &str = "/settings/";

const TRAY_ICON_ID: &str = "synara-tray";

#[derive(Clone, Serialize, serde::Deserialize)]
pub struct DesktopAgentActionPayload {
    id: String,
    title: String,
    kind: Option<String>,
    prompt: Option<String>,
    url: Option<String>,
    markdown: Option<String>,
}

#[derive(Clone, serde::Deserialize)]
pub struct DesktopShortcutConfig {
    pub show: String,
    pub later: String,
    pub notifications: String,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationPayload {
    pub title: String,
    pub body: Option<String>,
    pub route: Option<String>,
}

#[derive(Clone, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopShortcutApplyResult {
    pub success: bool,
    pub state: DesktopShortcutApplyState,
    pub message: String,
    pub fallback_command: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DesktopShortcutApplyState {
    Active,
    PermissionNeeded,
    Unsupported,
    Failed,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIntegrationCheck {
    pub name: String,
    pub ready: bool,
    pub supported: bool,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIntegrationStatus {
    pub platform: &'static str,
    pub desktop_environment: String,
    pub session_type: String,
    pub distro_id: String,
    pub distro_name: String,
    pub distro_version: String,
    pub build_identity: String,
    pub tray: DesktopIntegrationCheck,
    pub notifications: DesktopIntegrationCheck,
    pub global_shortcuts: DesktopIntegrationCheck,
    pub file_portal: DesktopIntegrationCheck,
    pub media_portal: DesktopIntegrationCheck,
}

#[derive(Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopTrayState {
    pub unread_count: i64,
    pub highlight_count: i64,
    pub later_count: i64,
    pub notification_inbox_count: i64,
    pub do_not_disturb: bool,
}

static LAST_SHORTCUT_APPLY_STATE: OnceLock<Mutex<Option<DesktopShortcutApplyState>>> = OnceLock::new();

#[derive(Clone, Serialize)]
struct DesktopAgentActionEvent {
    action: DesktopAgentActionPayload,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPerformanceCapabilities {
    platform: &'static str,
    app_version: &'static str,
    build_revision: &'static str,
    build_branch: &'static str,
    build_label: String,
}

const DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS: usize = 1024;
const DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS: usize = 16_384;
const DESKTOP_NOTIFICATION_MAX_TITLE_CHARS: usize = 120;
const DESKTOP_NOTIFICATION_MAX_BODY_CHARS: usize = 500;
const MAX_DESKTOP_ROUTE_CHARS: usize = 2_048;
const ALLOWED_SHORTCUT_LEN: usize = 128;
const UNKNOWN_INTEGRATION_VALUE: &str = "unknown";
const MAX_TRAY_COUNT: i64 = 9_999;
const ALLOWED_AGENT_ACTION_KIND: &[&str] = &[
    "agent",
    "copy",
    "continue",
    "export",
    "prompt",
    "regenerate",
    "run",
    "open",
    "open_url",
];

fn trim_shortcut(value: String) -> String {
    value.trim().replace(' ', "")
}

fn normalize_shortcut(shortcut: &str) -> String {
    trim_shortcut(shortcut.to_owned())
}

fn parse_shortcut(shortcut: &str) -> Result<Shortcut, String> {
    shortcut
        .parse::<Shortcut>()
        .map_err(|error| format!("Failed to parse shortcut '{shortcut}': {error}"))
}

fn validate_shortcuts(shortcuts: &DesktopShortcutConfig) -> Result<DesktopShortcutConfig, String> {
    let show = normalize_shortcut(&shortcuts.show);
    let later = normalize_shortcut(&shortcuts.later);
    let notifications = normalize_shortcut(&shortcuts.notifications);
    if show.is_empty() || later.is_empty() || notifications.is_empty() {
        return Err("Shortcut values cannot be empty".to_string());
    }
    if show.len() > ALLOWED_SHORTCUT_LEN
        || later.len() > ALLOWED_SHORTCUT_LEN
        || notifications.len() > ALLOWED_SHORTCUT_LEN
    {
        return Err("Shortcut values are too long".to_string());
    }

    let parsed_show = parse_shortcut(&show)?;
    let parsed_later = parse_shortcut(&later)?;
    let parsed_notifications = parse_shortcut(&notifications)?;
    if parsed_show == parsed_later
        || parsed_show == parsed_notifications
        || parsed_later == parsed_notifications
    {
        return Err("Shortcut values must be unique".to_string());
    }

    Ok(DesktopShortcutConfig {
        show,
        later,
        notifications,
    })
}

fn truncate_text(value: String, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn sanitize_action_text(value: String, max_chars: usize) -> String {
    truncate_text(value.trim().to_string(), max_chars)
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

    let route = notification.route.and_then(|value| sanitize_route(value).ok());

    Ok(DesktopNotificationPayload { title, body, route })
}

pub fn is_safe_external_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    match url.scheme() {
        "https" | "http" => {
            url.host_str().is_some() && url.username().is_empty() && url.password().is_none()
        }
        "mailto" | "matrix" => true,
        _ => false,
    }
}

fn is_safe_agent_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
}

fn sanitize_route(route: String) -> Result<String, String> {
    let route = sanitize_action_text(route, MAX_DESKTOP_ROUTE_CHARS);
    if route.is_empty() {
        return Err("Route cannot be empty".to_owned());
    }
    if route.contains("://") {
        return Err("Route must be an internal app route".to_owned());
    }
    if !route.starts_with('/') && !route.starts_with('#') {
        return Err("Route must start with / or #".to_owned());
    }
    Ok(route)
}

fn clamp_count(value: i64) -> i64 {
    match value {
        value if value < 0 => 0,
        value if value > MAX_TRAY_COUNT => MAX_TRAY_COUNT,
        value => value,
    }
}

fn last_shortcut_state() -> &'static Mutex<Option<DesktopShortcutApplyState>> {
    LAST_SHORTCUT_APPLY_STATE.get_or_init(|| Mutex::new(None))
}

fn set_last_shortcut_apply_state(state: DesktopShortcutApplyState) {
    if let Ok(mut guard) = last_shortcut_state().lock() {
        *guard = Some(state);
    }
}

fn read_last_shortcut_apply_state() -> Option<DesktopShortcutApplyState> {
    last_shortcut_state()
        .lock()
        .ok()
        .and_then(|state| state.clone())
}

fn is_kde() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.to_ascii_lowercase().contains("kde"))
        .unwrap_or(false)
}

fn is_wayland() -> bool {
    if env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }

    env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
}

fn is_kde_wayland_session() -> bool {
    is_kde() && is_wayland()
}

fn detect_session_type() -> String {
    if is_wayland() {
        return "wayland".to_owned();
    }
    if env::var("DISPLAY").is_ok() {
        return "x11".to_owned();
    }
    env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| UNKNOWN_INTEGRATION_VALUE.to_owned())
}

fn desktop_environment_label() -> String {
    if is_kde_wayland_session() {
        return "KDE Plasma Wayland".to_owned();
    }
    if is_kde() {
        return "KDE".to_owned();
    }
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.trim().to_owned())
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| UNKNOWN_INTEGRATION_VALUE.to_owned())
}

fn parse_os_release_field(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if !line.starts_with(&prefix) {
            continue;
        }
        let value = line.trim_start_matches(&prefix).trim();
        return Some(unquote_os_release_value(value));
    }
    None
}

fn unquote_os_release_value(value: &str) -> String {
    let stripped = value.trim().trim_matches('"');
    stripped.to_owned()
}

fn detect_os_release() -> (String, String, String) {
    let default = UNKNOWN_INTEGRATION_VALUE.to_owned();
    let path = Path::new("/etc/os-release");
    if !path.exists() {
        return (default.clone(), default.clone(), default);
    }

    let Ok(contents) = fs::read_to_string(path) else {
        return (default.clone(), default.clone(), default);
    };

    let distro_id = parse_os_release_field(&contents, "ID").unwrap_or_else(|| default.clone());
    let distro_name = parse_os_release_field(&contents, "NAME").unwrap_or_else(|| distro_id.clone());
    let distro_version = parse_os_release_field(&contents, "VERSION_ID").unwrap_or_else(|| default.clone());
    (distro_id, distro_name, distro_version)
}

fn dir_has_fragment(path: &str, fragment: &str) -> bool {
    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    let fragment = fragment.to_ascii_lowercase();
    entries.any(|entry| {
        let Ok(entry) = entry else {
            return false;
        };
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        name.contains(&fragment)
    })
}

fn has_media_portal_backend() -> bool {
    dir_has_fragment("/usr/share/xdg-desktop-portal/portals", "screencast")
        || dir_has_fragment("/usr/share/dbus-1/services", "screencast")
        || dir_has_fragment("/usr/share/dbus-1/services", "camera")
        || dir_has_fragment("/usr/share/xdg-desktop-portal/portals", "screenshot")
}

fn has_file_portal_backend() -> bool {
    dir_has_fragment("/usr/share/xdg-desktop-portal/portals", "file")
        || dir_has_fragment("/usr/share/dbus-1/services", "org.freedesktop.portal.files")
        || dir_has_fragment("/usr/share/dbus-1/services", "filechooser")
}

fn shortcut_apply_state_message(state: DesktopShortcutApplyState) -> &'static str {
    match state {
        DesktopShortcutApplyState::Active => "Desktop shortcuts are active.",
        DesktopShortcutApplyState::PermissionNeeded => {
            "Shortcut registration needs permission on this desktop session."
        }
        DesktopShortcutApplyState::Unsupported => "Desktop shortcuts are unsupported in this environment.",
        DesktopShortcutApplyState::Failed => "Desktop shortcut registration failed.",
    }
}

fn desktop_shortcut_fallback_command() -> Option<String> {
    if is_kde_wayland_session() {
        return Some("Open System Settings > Shortcuts and create a custom shortcut for Synara.".to_string());
    }
    None
}

fn shortcut_result(
    state: DesktopShortcutApplyState,
    message: Option<String>,
    fallback_command: Option<String>,
) -> DesktopShortcutApplyResult {
    let fallback_command = if matches!(state, DesktopShortcutApplyState::PermissionNeeded) {
        Some("Open System Settings > Shortcuts and create a custom shortcut for Synara.".to_string())
    } else {
        fallback_command
    };

    DesktopShortcutApplyResult {
        success: matches!(state, DesktopShortcutApplyState::Active),
        state,
        message: message.unwrap_or_else(|| shortcut_apply_state_message(state).to_owned()),
        fallback_command,
    }
}

fn shortcut_state_from_error(error: &str) -> DesktopShortcutApplyState {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("permission") || normalized.contains("denied") {
        DesktopShortcutApplyState::PermissionNeeded
    } else if normalized.contains("not supported") || normalized.contains("unsupported") {
        DesktopShortcutApplyState::Unsupported
    } else {
        DesktopShortcutApplyState::Failed
    }
}

fn tray_route_labels(state: &DesktopTrayState) -> [String; 5] {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    let notifications = clamp_count(state.notification_inbox_count);
    let do_not_disturb = state.do_not_disturb;
    let summary = format!(
        "Unread: {unread} | Highlights: {highlights} | Later: {later} | Notifications: {notifications}"
    );
    let later_label = format!("Later ({later})");
    let notifications_label = format!("Notifications ({notifications})");
    let dnd_label = if do_not_disturb {
        "Do Not Disturb: On"
    } else {
        "Do Not Disturb: Off"
    };
    let integration_label = "Desktop Integration";
    [
        summary,
        later_label,
        notifications_label,
        dnd_label.to_owned(),
        integration_label.to_owned(),
    ]
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>, state: &DesktopTrayState) -> tauri::Result<Menu<R>> {
    let [unread_summary, later_label, notifications_label, dnd_label, integration_label] =
        tray_route_labels(state);

    let show = MenuItem::with_id(
        app,
        MENU_SHOW,
        "Show Synara",
        true,
        Some("CmdOrCtrl+Shift+C"),
    )?;
    let unread_summary = MenuItem::with_id(
        app,
        MENU_UNREAD_SUMMARY,
        unread_summary.as_str(),
        false,
        None::<&str>,
    )?;
    let later = MenuItem::with_id(app, MENU_LATER, later_label.as_str(), true, Some("CmdOrCtrl+Shift+L"))?;
    let notifications = MenuItem::with_id(
        app,
        MENU_NOTIFICATIONS,
        notifications_label.as_str(),
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let desktop_integration = MenuItem::with_id(
        app,
        MENU_DESKTOP_INTEGRATION,
        integration_label.as_str(),
        true,
        None::<&str>,
    )?;
    let dnd = MenuItem::with_id(
        app,
        MENU_DND_TOGGLE,
        dnd_label.as_str(),
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let build_item = MenuItem::with_id(
        app,
        MENU_BUILD_INFO,
        build_info::menu_label(),
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Synara", true, Some("CmdOrCtrl+Q"))?;

    #[cfg(not(target_os = "linux"))]
    let menu = Menu::with_items(app, &[&show, &later, &notifications, &separator, &build_item, &quit])?;

    #[cfg(target_os = "linux")]
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &unread_summary,
            &later,
            &notifications,
            &desktop_integration,
            &dnd,
            &separator,
            &build_item,
            &quit,
        ],
    )?;

    Ok(menu)
}

fn tray_tooltip(state: &DesktopTrayState) -> String {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    format!("Synara — {unread} unread ({highlights} highlights), {later} later")
}

fn sanitize_agent_action_payload(
    mut action: DesktopAgentActionPayload,
) -> Result<DesktopAgentActionPayload, String> {
    action.id = sanitize_action_text(action.id, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
    if action.id.is_empty() {
        return Err("Agent action payload missing action id".to_owned());
    }

    action.title = sanitize_action_text(action.title, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
    if action.title.is_empty() {
        return Err("Agent action payload missing title".to_owned());
    }

    if let Some(kind) = action.kind.take() {
        let normalized = kind.trim().to_lowercase();
        if !ALLOWED_AGENT_ACTION_KIND.contains(&normalized.as_str()) {
            return Err("Agent action kind is not allowed".to_owned());
        }
        action.kind = Some(normalized);
    }

    if let Some(url) = action.url.take() {
        if !is_safe_agent_url(&url) {
            return Err("Agent action URL must use https".to_owned());
        }
        action.url = Some(sanitize_action_text(
            url,
            DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS,
        ));
    }

    if let Some(prompt) = action.prompt.take() {
        let sanitized = sanitize_action_text(prompt, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
        if !sanitized.is_empty() {
            action.prompt = Some(sanitized);
        }
    }

    if let Some(markdown) = action.markdown.take() {
        let sanitized = truncate_text(markdown, DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS);
        if !sanitized.is_empty() {
            action.markdown = Some(sanitized);
        }
    }

    if action.url.is_none() && action.prompt.is_none() && action.markdown.is_none() {
        return Err("Agent action payload missing runnable payload".to_owned());
    }

    Ok(action)
}

fn extract_agent_action_copy_text(action: &DesktopAgentActionPayload) -> Option<String> {
    if let Some(markdown) = action.markdown.as_deref() {
        return Some(markdown.to_owned());
    }

    if let Some(prompt) = action.prompt.as_deref() {
        return Some(prompt.to_owned());
    }

    if !action.title.is_empty() {
        return Some(action.title.clone());
    }

    None
}

fn handle_agent_action_locally<R: Runtime>(
    app: &AppHandle<R>,
    action: &DesktopAgentActionPayload,
) -> bool {
    match action.kind.as_deref() {
        Some("copy") => {
            let Some(copy_text) = extract_agent_action_copy_text(action) else {
                return false;
            };
            app.clipboard().write_text(copy_text).is_ok()
        }
        Some("open") | Some("open_url") => action
            .url
            .as_ref()
            .is_some_and(|url| app.opener().open_url(url.as_str(), None::<&str>).is_ok()),
        None => action
            .url
            .as_ref()
            .is_some_and(|url| app.opener().open_url(url.as_str(), None::<&str>).is_ok()),
        _ => false,
    }
}

fn is_supported_agent_action(action: &DesktopAgentActionPayload) -> bool {
    match (&action.kind, &action.url) {
        (Some(kind), _) => ALLOWED_AGENT_ACTION_KIND.contains(&kind.as_str()),
        (None, Some(_)) => true,
        _ => false,
    }
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        window.hide()?;
    }
    Ok(())
}

pub fn navigate_main_window<R: Runtime>(app: &AppHandle<R>, route: &str) -> tauri::Result<()> {
    show_main_window(app)?;

    if let Some(window) = main_window(app) {
        let hash = format!("#{}", route.trim_start_matches('#'));
        let hash_json = serde_json::to_string(&hash).unwrap_or_else(|_| "\"#/\"".to_owned());
        window.eval(format!("window.location.hash = {};", hash_json))?;
    }

    Ok(())
}

pub fn set_badge_count<R: Runtime>(app: &AppHandle<R>, count: Option<i64>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        let normalized_count = count.filter(|value| *value > 0);
        window.set_badge_count(normalized_count)?;

        #[cfg(target_os = "macos")]
        {
            window.set_badge_label(normalized_count.map(|value| value.to_string()))?;
        }
    }
    Ok(())
}

pub fn performance_capabilities() -> DesktopPerformanceCapabilities {
    DesktopPerformanceCapabilities {
        platform: std::env::consts::OS,
        app_version: build_info::app_version(),
        build_revision: build_info::revision(),
        build_branch: build_info::branch(),
        build_label: build_info::label(),
    }
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let initial_state = DesktopTrayState {
        unread_count: 0,
        highlight_count: 0,
        later_count: 0,
        notification_inbox_count: 0,
        do_not_disturb: false,
    };
    let menu = build_tray_menu(app, &initial_state)?;

    let mut builder = TrayIconBuilder::with_id("synara-tray")
        .tooltip(&tray_tooltip(&initial_state))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event);

    #[cfg(target_os = "macos")]
    {
        let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
        builder = builder.icon(tray_icon).icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone()).icon_as_template(false);
    }

    builder.build(app)?;
    Ok(())
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let result = match event.id().as_ref() {
        MENU_SHOW => show_main_window(app),
        MENU_LATER => navigate_main_window(app, ROUTE_LATER),
        MENU_NOTIFICATIONS => navigate_main_window(app, ROUTE_NOTIFICATIONS),
        MENU_UNREAD_SUMMARY => navigate_main_window(app, ROUTE_HOME),
        MENU_DESKTOP_INTEGRATION => navigate_main_window(app, ROUTE_SETTINGS),
        MENU_DND_TOGGLE => Ok(()),
        MENU_BUILD_INFO => Ok(()),
        MENU_QUIT => {
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };

    if let Err(error) = result {
        eprintln!("failed to handle desktop menu event: {error}");
    }
}

#[tauri::command]
pub fn desktop_show(app: AppHandle) -> Result<(), String> {
    show_main_window(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_hide(app: AppHandle) -> Result<(), String> {
    hide_main_window(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_navigate(app: AppHandle, route: String) -> Result<(), String> {
    let route = sanitize_route(route)?;
    navigate_main_window(&app, &route).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_badge_count(app: AppHandle, count: i64) -> Result<(), String> {
    set_badge_count(&app, Some(count)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_shortcuts(app: AppHandle, shortcuts: DesktopShortcutConfig) -> DesktopShortcutApplyResult {
    let supported = cfg!(not(any(target_os = "android", target_os = "ios")));
    if !supported {
        return shortcut_result(
            DesktopShortcutApplyState::Unsupported,
            Some("Global shortcuts are not supported on this platform.".to_string()),
            None,
        );
    }

    let normalized = match validate_shortcuts(&shortcuts) {
        Ok(normalized) => normalized,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };

    let parsed_show = match parse_shortcut(&normalized.show) {
        Ok(value) => value,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };
    let parsed_later = match parse_shortcut(&normalized.later) {
        Ok(value) => value,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };
    let parsed_notifications = match parse_shortcut(&normalized.notifications) {
        Ok(value) => value,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };

    let mut route_by_id = HashMap::new();
    route_by_id.insert(parsed_show.id(), ROUTE_HOME);
    route_by_id.insert(parsed_later.id(), ROUTE_LATER);
    route_by_id.insert(parsed_notifications.id(), ROUTE_NOTIFICATIONS);

    let global_shortcut = app.global_shortcut();
    if let Err(error) = global_shortcut.unregister_all() {
        let state = shortcut_state_from_error(&error.to_string());
        let result = shortcut_result(
            state,
            Some(format!("Failed to clear previous shortcuts: {error}")),
            desktop_shortcut_fallback_command(),
        );
        set_last_shortcut_apply_state(state);
        return result;
    };

    if let Err(error) = global_shortcut.on_shortcuts(
        [
            normalized.show.as_str(),
            normalized.later.as_str(),
            normalized.notifications.as_str(),
        ],
        move |app: &AppHandle<tauri::Wry>, shortcut: &Shortcut, event: ShortcutEvent| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let Some(route) = route_by_id.get(&shortcut.id()) else {
                return;
            };
            if let Err(error) = navigate_main_window(app, route) {
                eprintln!("failed to handle desktop shortcut: {error}");
            }
        },
    ) {
        let state = shortcut_state_from_error(&error.to_string());
        let result = shortcut_result(
            state,
            Some(format!("Failed to register desktop shortcuts: {error}")),
            desktop_shortcut_fallback_command(),
        );
        set_last_shortcut_apply_state(state);
        return result;
    }

    set_last_shortcut_apply_state(DesktopShortcutApplyState::Active);
    shortcut_result(DesktopShortcutApplyState::Active, None, None)
}

#[tauri::command]
pub fn desktop_get_integration_status(app: AppHandle) -> DesktopIntegrationStatus {
    let (distro_id, distro_name, distro_version) = detect_os_release();
    let desktop_environment = desktop_environment_label();
    let session_type = detect_session_type();
    let tray = app
        .tray_by_id(TRAY_ICON_ID)
        .map(|_| DesktopIntegrationCheck {
            name: "Tray".to_string(),
            ready: true,
            supported: true,
            message: "Tray is available.".to_string(),
        })
        .unwrap_or_else(|| DesktopIntegrationCheck {
            name: "Tray".to_string(),
            ready: false,
            supported: false,
            message: "Tray is unavailable.".to_string(),
        });

    let notifications = app
        .notification()
        .permission_state()
        .map(|permission| permission.to_string().to_ascii_lowercase())
        .map(|permission| {
            let supported = !permission.is_empty();
            let ready = permission != "denied";
            let message = if ready {
                "Notification permission is active."
            } else {
                "Notifications are blocked by platform permission."
            };
            DesktopIntegrationCheck {
                name: "Notifications".to_string(),
                supported,
                ready,
                message: message.to_string(),
            }
        })
        .unwrap_or_else(|_| DesktopIntegrationCheck {
            name: "Notifications".to_string(),
            ready: false,
            supported: false,
            message: "Notification state could not be read.".to_string(),
        });

    let shortcut_state = read_last_shortcut_apply_state().unwrap_or_else(|| {
        if is_kde_wayland_session() {
            DesktopShortcutApplyState::Failed
        } else {
            DesktopShortcutApplyState::Active
        }
    });
    let global_shortcuts = DesktopIntegrationCheck {
        name: "Global Shortcuts".to_string(),
        supported: cfg!(not(any(target_os = "android", target_os = "ios"))),
        ready: matches!(shortcut_state, DesktopShortcutApplyState::Active),
        message: match shortcut_state {
            DesktopShortcutApplyState::Active => "Global shortcuts are active.".to_string(),
            DesktopShortcutApplyState::PermissionNeeded => {
                "Global shortcuts require permission in this desktop session.".to_string()
            }
            DesktopShortcutApplyState::Unsupported => {
                "Global shortcuts are unsupported in this build.".to_string()
            }
            DesktopShortcutApplyState::Failed => {
                if is_kde_wayland_session() {
                    "Global shortcuts may require permission on KDE Wayland.".to_string()
                } else {
                    "Global shortcuts not currently active.".to_string()
                }
            }
        },
    };

    let file_portal_available = has_file_portal_backend();
    let media_portal_available = has_media_portal_backend();
    let file_portal = DesktopIntegrationCheck {
        name: "File Portal".to_string(),
        supported: true,
        ready: file_portal_available,
        message: if file_portal_available {
            "File portal backend detected."
        } else {
            "File portal backend not detected."
        }
        .to_string(),
    };
    let media_portal = DesktopIntegrationCheck {
        name: "Media Portal".to_string(),
        supported: true,
        ready: media_portal_available,
        message: if media_portal_available {
            "Media portal backend detected."
        } else {
            "Media portal backend not detected."
        }
        .to_string(),
    };

    DesktopIntegrationStatus {
        platform: std::env::consts::OS,
        desktop_environment,
        session_type,
        distro_id,
        distro_name,
        distro_version,
        build_identity: build_info::menu_label(),
        tray,
        notifications,
        global_shortcuts,
        file_portal,
        media_portal,
    }
}

#[tauri::command]
pub fn desktop_update_tray_state(
    app: AppHandle,
    state: DesktopTrayState,
) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        let state = DesktopTrayState {
            unread_count: clamp_count(state.unread_count),
            highlight_count: clamp_count(state.highlight_count),
            later_count: clamp_count(state.later_count),
            notification_inbox_count: clamp_count(state.notification_inbox_count),
            do_not_disturb: state.do_not_disturb,
        };
        let menu = build_tray_menu(&app, &state).map_err(|error| error.to_string())?;
        tray.set_menu(Some(menu)).map_err(|error| error.to_string())?;
        tray.set_tooltip(Some(tray_tooltip(&state)))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn desktop_get_notification_permission(app: AppHandle) -> Result<String, String> {
    app.notification()
        .permission_state()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_request_notification_permission(app: AppHandle) -> Result<String, String> {
    app.notification()
        .request_permission()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_notify(
    app: AppHandle,
    notification: DesktopNotificationPayload,
) -> Result<bool, String> {
    let notification = sanitize_notification_payload(notification)?;
    let mut builder = app.notification().builder().title(notification.title);
    if let Some(body) = notification.body {
        builder = builder.body(body);
    }
    builder.show().map_err(|error| error.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn desktop_get_performance_capabilities() -> DesktopPerformanceCapabilities {
    performance_capabilities()
}

#[tauri::command]
pub fn desktop_agent_action(
    app: AppHandle,
    action: DesktopAgentActionPayload,
) -> Result<bool, String> {
    let action = sanitize_agent_action_payload(action).map_err(|error| error.to_string())?;
    if !is_supported_agent_action(&action) {
        return Ok(false);
    }

    if handle_agent_action_locally(&app, &action) {
        return Ok(true);
    }

    app.emit("synara://agent-action", DesktopAgentActionEvent { action })
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn validate_shortcuts_accepts_valid_input() {
        let normalized = validate_shortcuts(&DesktopShortcutConfig {
            show: "cmd+shift+c".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: " CmdOrCtrl+Shift+N ".to_string(),
        })
        .expect("shortcuts should validate");

        assert_eq!(normalized.show, "cmd+shift+c");
        assert_eq!(normalized.later, "CmdOrCtrl+Shift+L");
        assert_eq!(normalized.notifications, "CmdOrCtrl+Shift+N");
    }

    #[test]
    fn validate_shortcuts_rejects_duplicate_shortcuts() {
        let result = validate_shortcuts(&DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+C".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn validate_shortcuts_rejects_invalid_shortcut() {
        let result = validate_shortcuts(&DesktopShortcutConfig {
            show: "Ctrl+".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_allows_https_urls() {
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("https://example.org/action".to_owned()),
            markdown: None,
        })
        .expect("action payload should pass");

        assert_eq!(payload.id, "abc");
        assert_eq!(payload.url.as_deref(), Some("https://example.org/action"));
    }

    #[test]
    fn sanitize_action_payload_rejects_plain_http_urls() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("http://example.org/action".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_credentialed_urls() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("https://user:pass@example.org/action".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_disallowed_scheme() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("file:///tmp/test".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_unsupported_kind() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("malicious".to_owned()),
            prompt: Some("Run local tool".to_owned()),
            url: None,
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_requires_payload() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: None,
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn supported_agent_action_detects_no_kind_with_url() {
        let payload = sanitize_action_payload_with_no_kind(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: None,
            prompt: None,
            url: Some("https://example.org/action".to_owned()),
            markdown: None,
        });

        assert!(is_supported_agent_action(&payload));
    }

    #[test]
    fn extract_copy_text_prefers_markdown() {
        let payload = sanitize_action_payload_with_no_kind(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("copy".to_owned()),
            prompt: Some("Prompt".to_owned()),
            url: None,
            markdown: Some("```\nBlock\n```".to_owned()),
        });

        assert_eq!(
            extract_agent_action_copy_text(&payload),
            Some("```\nBlock\n```".to_owned())
        );
    }

    #[test]
    fn performance_capabilities_reflect_platform_support() {
        let capabilities = performance_capabilities();
        assert_eq!(capabilities.platform, std::env::consts::OS);
        assert_eq!(capabilities.app_version, env!("CARGO_PKG_VERSION"));
        assert!(!capabilities.build_revision.is_empty());
        assert!(!capabilities.build_branch.is_empty());
        assert!(capabilities.build_label.contains(capabilities.app_version));
    }

    #[test]
    fn external_url_filter_allows_http_links_and_blocks_scriptable_schemes() {
        assert!(is_safe_external_url("https://example.org/path"));
        assert!(is_safe_external_url("http://example.org/path"));
        assert!(is_safe_external_url("mailto:test@example.org"));
        assert!(!is_safe_external_url("javascript:alert(1)"));
        assert!(!is_safe_external_url("file:///Users/example/.ssh/id_rsa"));
        assert!(!is_safe_external_url("https://user:pass@example.org/"));
    }

    #[test]
    fn sanitize_route_allows_only_internal_routes() {
        assert_eq!(
            sanitize_route("/inbox/later/".to_owned()).unwrap(),
            "/inbox/later/"
        );
        assert_eq!(
            sanitize_route("#/room/abc".to_owned()).unwrap(),
            "#/room/abc"
        );
        assert!(sanitize_route("https://example.org".to_owned()).is_err());
        assert!(sanitize_route("room/abc".to_owned()).is_err());
    }

    #[test]
    fn sanitize_notification_payload_rejects_empty_title() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            title: "  ".to_owned(),
            body: Some("Body".to_owned()),
            route: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_notification_payload_truncates_body() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("a".repeat(DESKTOP_NOTIFICATION_MAX_BODY_CHARS + 10)),
            route: Some("/inbox/".to_owned()),
        })
        .expect("notification payload should pass");

        assert_eq!(
            payload.body.unwrap().chars().count(),
            DESKTOP_NOTIFICATION_MAX_BODY_CHARS
        );
    }

    #[test]
    fn sanitize_notification_payload_accepts_safe_route_and_strips_invalid_route() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("/inbox/notifications/".to_owned()),
        })
        .expect("notification payload should pass");
        assert_eq!(
            payload.route,
            Some("/inbox/notifications/".to_string())
        );

        let invalid = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("https://evil.example.com".to_owned()),
        })
        .expect("notification payload should pass without route");
        assert_eq!(invalid.route, None);
    }

    #[test]
    fn parse_os_release_detects_cachyos_metadata() {
        let data = r#"
ID=cachyos
NAME="CachyOS"
VERSION_ID=24
"#;

        assert_eq!(parse_os_release_field(data, "ID").unwrap_or_else(|| "".to_owned()), "cachyos");
        assert_eq!(
            parse_os_release_field(data, "NAME").unwrap_or_else(|| "".to_owned()),
            "CachyOS"
        );
        assert_eq!(
            parse_os_release_field(data, "VERSION_ID").unwrap_or_else(|| "".to_owned()),
            "24"
        );
    }

    #[test]
    fn detect_integration_environment_falls_back_for_absent_values() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_session_type = std::env::var("XDG_SESSION_TYPE").ok();
        let original_display = std::env::var("DISPLAY").ok();
        let original_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        std::env::remove_var("XDG_CURRENT_DESKTOP");
        std::env::remove_var("XDG_SESSION_TYPE");
        std::env::remove_var("DISPLAY");
        std::env::remove_var("WAYLAND_DISPLAY");

        assert_eq!(desktop_environment_label(), UNKNOWN_INTEGRATION_VALUE.to_owned());
        assert_eq!(detect_session_type(), UNKNOWN_INTEGRATION_VALUE.to_owned());

        if let Some(value) = original_desktop {
            std::env::set_var("XDG_CURRENT_DESKTOP", value);
        }
        if let Some(value) = original_session_type {
            std::env::set_var("XDG_SESSION_TYPE", value);
        }
        if let Some(value) = original_display {
            std::env::set_var("DISPLAY", value);
        }
        if let Some(value) = original_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", value);
        }
    }

    #[test]
    fn detect_cachyos_like_desktop_is_kde_wayland_when_flags_match() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_wayland = std::env::var("WAYLAND_DISPLAY").ok();
        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");

        assert!(is_kde_wayland_session());
        assert_eq!(desktop_environment_label(), "KDE Plasma Wayland");
        assert_eq!(detect_session_type(), "wayland");

        if let Some(value) = original_desktop {
            std::env::set_var("XDG_CURRENT_DESKTOP", value);
        } else {
            std::env::remove_var("XDG_CURRENT_DESKTOP");
        }
        if let Some(value) = original_wayland {
            std::env::set_var("WAYLAND_DISPLAY", value);
        } else {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
    }

    #[test]
    fn shortcut_state_classifier_detects_permission_errors_and_result_shapes() {
        assert_eq!(
            shortcut_state_from_error("failed with denied"),
            DesktopShortcutApplyState::PermissionNeeded
        );
        assert_eq!(
            shortcut_state_from_error("shortcut unsupported on this build"),
            DesktopShortcutApplyState::Unsupported
        );

        let result = shortcut_result(
            DesktopShortcutApplyState::PermissionNeeded,
            None,
            None,
        );
        assert!(!result.success);
        assert_eq!(
            result.state,
            DesktopShortcutApplyState::PermissionNeeded
        );
        assert!(result.fallback_command.is_some());
    }

    #[test]
    fn tray_state_fields_are_clamped() {
        assert_eq!(clamp_count(-1), 0);
        assert_eq!(clamp_count(15_000), 9_999);
        assert_eq!(clamp_count(23), 23);

        let labels = tray_route_labels(&DesktopTrayState {
            unread_count: -5,
            highlight_count: 12_000,
            later_count: 3,
            notification_inbox_count: -9,
            do_not_disturb: true,
        });
        assert!(labels[0].contains("Unread: 0"));
        assert!(labels[0].contains("Highlights: 9999"));
        assert!(labels[0].contains("Later: 3"));
        assert!(labels[0].contains("Notifications: 0"));
    }

    fn sanitize_action_payload_with_no_kind(
        action: DesktopAgentActionPayload,
    ) -> DesktopAgentActionPayload {
        sanitize_agent_action_payload(action).expect("action payload should pass")
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn global_shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_global_shortcut::ShortcutState;

    tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([
            "CmdOrCtrl+Shift+C",
            "CmdOrCtrl+Shift+L",
            "CmdOrCtrl+Shift+N",
        ])
        .expect("desktop global shortcuts must parse")
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let result = match shortcut.to_string().as_str() {
                "CmdOrCtrl+Shift+C" => navigate_main_window(app, ROUTE_HOME),
                "CmdOrCtrl+Shift+L" => navigate_main_window(app, ROUTE_LATER),
                "CmdOrCtrl+Shift+N" => navigate_main_window(app, ROUTE_NOTIFICATIONS),
                _ => Ok(()),
            };

            if let Err(error) = result {
                eprintln!("failed to handle desktop shortcut: {error}");
            }
        })
        .build()
}
