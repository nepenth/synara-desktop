use serde::Serialize;
use std::collections::HashMap;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};
use tauri_plugin_opener::OpenerExt;

use crate::build_info;

pub const MAIN_WINDOW_LABEL: &str = "main";

const MENU_SHOW: &str = "desktop.show";
const MENU_LATER: &str = "desktop.later";
const MENU_NOTIFICATIONS: &str = "desktop.notifications";
const MENU_CHECK_UPDATES: &str = "desktop.check-updates";
const MENU_BUILD_INFO: &str = "desktop.build-info";
const MENU_QUIT: &str = "desktop.quit";

const ROUTE_HOME: &str = "/";
const ROUTE_LATER: &str = "/inbox/later/";
const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";

#[derive(Clone, Serialize)]
struct DesktopActionPayload<'a> {
    action: &'a str,
}

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
const ALLOWED_SHORTCUT_LEN: usize = 128;
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
        if !url.starts_with("https://") {
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

pub fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        if window.is_visible()? {
            window.hide()?;
        } else {
            window.show()?;
            window.unminimize()?;
            window.set_focus()?;
        }
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

pub fn emit_check_updates<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    show_main_window(app)?;
    app.emit(
        "cinny://desktop-action",
        DesktopActionPayload {
            action: "check-updates",
        },
    )?;
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
    let show = MenuItem::with_id(
        app,
        MENU_SHOW,
        "Show Synara",
        true,
        Some("CmdOrCtrl+Shift+C"),
    )?;
    let later = MenuItem::with_id(app, MENU_LATER, "Later", true, Some("CmdOrCtrl+Shift+L"))?;
    let notifications = MenuItem::with_id(
        app,
        MENU_NOTIFICATIONS,
        "Notifications",
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let check_updates = MenuItem::with_id(
        app,
        MENU_CHECK_UPDATES,
        "Check for Updates",
        true,
        None::<&str>,
    )?;
    let build_item = MenuItem::with_id(
        app,
        MENU_BUILD_INFO,
        build_info::menu_label(),
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Synara", true, Some("CmdOrCtrl+Q"))?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &later,
            &notifications,
            &separator,
            &check_updates,
            &build_item,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("synara-tray")
        .tooltip("Synara")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = toggle_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone()).icon_as_template(true);
    }

    builder.build(app)?;
    Ok(())
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let result = match event.id().as_ref() {
        MENU_SHOW => show_main_window(app),
        MENU_LATER => navigate_main_window(app, ROUTE_LATER),
        MENU_NOTIFICATIONS => navigate_main_window(app, ROUTE_NOTIFICATIONS),
        MENU_CHECK_UPDATES => emit_check_updates(app),
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
    navigate_main_window(&app, &route).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_badge_count(app: AppHandle, count: i64) -> Result<(), String> {
    set_badge_count(&app, Some(count)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_shortcuts(
    app: AppHandle,
    shortcuts: DesktopShortcutConfig,
) -> Result<bool, String> {
    let normalized = validate_shortcuts(&shortcuts)?;
    let parsed_show = parse_shortcut(&normalized.show)?;
    let parsed_later = parse_shortcut(&normalized.later)?;
    let parsed_notifications = parse_shortcut(&normalized.notifications)?;

    let mut route_by_id = HashMap::new();
    route_by_id.insert(parsed_show.id(), ROUTE_HOME);
    route_by_id.insert(parsed_later.id(), ROUTE_LATER);
    route_by_id.insert(parsed_notifications.id(), ROUTE_NOTIFICATIONS);

    let global_shortcut = app.global_shortcut();
    global_shortcut
        .unregister_all()
        .map_err(|error: tauri_plugin_global_shortcut::Error| error.to_string())?;

    global_shortcut
        .on_shortcuts(
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
        )
        .map_err(|error: tauri_plugin_global_shortcut::Error| error.to_string())?;

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

    app.emit("cinny://agent-action", DesktopAgentActionEvent { action })
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

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
