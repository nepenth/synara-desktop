use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

use crate::build_info;
use crate::desktop_sanitize::sanitize_route;
#[cfg(any(target_os = "windows", test))]
use crate::desktop_secret_store::DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED;
use crate::desktop_secret_store::{bridge_supports_secure_secret_store, DesktopSecretStoreStatus};
#[cfg(test)]
use crate::desktop_secret_store::{
    unavailable_secret_store_status, DESKTOP_SECRET_STORE_BACKEND_NONE,
};
use crate::desktop_shortcuts::{
    desktop_set_shortcuts as apply_desktop_shortcuts_command, DesktopShortcutApplyResult,
    DesktopShortcutConfig,
};
use crate::desktop_tray::{self, DesktopTrayState};
use crate::desktop_url;

pub const MAIN_WINDOW_LABEL: &str = "main";

pub(crate) const ROUTE_HOME: &str = "/";
pub(crate) const ROUTE_LATER: &str = "/inbox/later/";
pub(crate) const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";
pub(crate) const ROUTE_SETTINGS: &str = "/settings/";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPerformanceCapabilities {
    platform: &'static str,
    app_version: &'static str,
    build_revision: &'static str,
    build_branch: &'static str,
    build_label: String,
    webview_engine: &'static str,
    hardware_acceleration_policy: String,
    smooth_scrolling_enabled: Option<bool>,
    software_rendering_override_detected: bool,
    dmabuf_fast_path_disabled: bool,
}

/// User-clicked external URLs are handed to the OS browser, not fetched by Synara.
/// Block local file/code schemes and embedded credentials, but allow ordinary
/// http(s) links including LAN/internal hosts users intentionally click.
pub fn is_safe_external_url(value: &str) -> bool {
    desktop_url::is_safe_external_url(value)
}

#[tauri::command]
pub fn desktop_open_external_url<R: Runtime>(app: AppHandle<R>, url: String) -> bool {
    if !is_safe_external_url(&url) {
        return false;
    }

    match app.opener().open_url(url, None::<&str>) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("[synara] Failed to open external URL: {error}");
            false
        }
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

pub fn performance_capabilities() -> DesktopPerformanceCapabilities {
    let webview = crate::desktop_webview_performance::capabilities();
    DesktopPerformanceCapabilities {
        platform: std::env::consts::OS,
        app_version: build_info::app_version(),
        build_revision: build_info::revision(),
        build_branch: build_info::branch(),
        build_label: build_info::label(),
        webview_engine: webview.webview_engine,
        hardware_acceleration_policy: webview.hardware_acceleration_policy,
        smooth_scrolling_enabled: webview.smooth_scrolling_enabled,
        software_rendering_override_detected: webview.software_rendering_override_detected,
        dmabuf_fast_path_disabled: webview.dmabuf_fast_path_disabled,
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

/// Window controls for the in-app titlebar (Linux borderless mode).
/// The main window always exists when the renderer is alive; a missing
/// window fails closed rather than inventing window state.
#[tauri::command]
pub fn desktop_window_minimize(app: AppHandle) -> Result<(), String> {
    main_window(&app)
        .ok_or_else(|| "No native main window is active.".to_owned())
        .and_then(|window| window.minimize().map_err(|error| error.to_string()))
}

#[tauri::command]
pub fn desktop_window_toggle_maximize(app: AppHandle) -> Result<bool, String> {
    let window = main_window(&app).ok_or_else(|| "No native main window is active.".to_owned())?;
    let maximized = window.is_maximized().map_err(|error| error.to_string())?;
    if maximized {
        window.unmaximize().map_err(|error| error.to_string())?;
        Ok(false)
    } else {
        window.maximize().map_err(|error| error.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
pub fn desktop_window_close(app: AppHandle) -> Result<(), String> {
    main_window(&app)
        .ok_or_else(|| "No native main window is active.".to_owned())
        .and_then(|window| window.close().map_err(|error| error.to_string()))
}

#[tauri::command]
pub fn desktop_navigate(app: AppHandle, route: String) -> Result<(), String> {
    let route = sanitize_route(route)?;
    navigate_main_window(&app, &route).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_badge_count(app: AppHandle, count: i64) -> Result<(), String> {
    desktop_tray::set_badge_count(&app, Some(count)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_shortcuts(
    app: AppHandle,
    shortcuts: DesktopShortcutConfig,
) -> DesktopShortcutApplyResult {
    apply_desktop_shortcuts_command(&app, shortcuts)
}

pub fn desktop_bridge_supports_secure_secret_store() -> bool {
    bridge_supports_secure_secret_store(&crate::desktop_secret_store::platform_secret_store_status())
}

#[tauri::command]
pub fn desktop_secret_store_status() -> DesktopSecretStoreStatus {
    crate::desktop_secret_store::platform_secret_store_status()
}

#[tauri::command]
pub fn desktop_update_tray_state(app: AppHandle, state: DesktopTrayState) -> Result<bool, String> {
    desktop_tray::update_tray_state(app, state)
}

#[tauri::command]
pub fn desktop_get_performance_capabilities() -> DesktopPerformanceCapabilities {
    performance_capabilities()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_capabilities_reflect_platform_support() {
        let capabilities = performance_capabilities();
        assert_eq!(capabilities.platform, std::env::consts::OS);
        assert_eq!(capabilities.app_version, env!("CARGO_PKG_VERSION"));
        assert!(!capabilities.build_revision.is_empty());
        assert!(!capabilities.build_branch.is_empty());
        assert!(capabilities.build_label.contains(capabilities.app_version));
        assert!(!capabilities.webview_engine.is_empty());
        assert!(!capabilities.hardware_acceleration_policy.is_empty());
    }

    #[test]
    fn external_url_filter_allows_user_clicked_http_https_links() {
        assert!(is_safe_external_url("https://example.org/path"));
        assert!(is_safe_external_url("http://example.org/path"));
        assert!(is_safe_external_url("http://127.0.0.1:8080"));
        assert!(is_safe_external_url("http://localhost:8080"));
        assert!(is_safe_external_url("https://192.168.1.1/"));
        assert!(is_safe_external_url(
            "https://169.254.169.254/latest/meta-data/"
        ));
        assert!(is_safe_external_url("https://metadata.google.internal/"));
        assert!(is_safe_external_url("https://app.local/"));
        assert!(is_safe_external_url("mailto:test@example.org"));
        assert!(is_safe_external_url("matrix:r/#room:example.org"));
        assert!(!is_safe_external_url("javascript:alert(1)"));
        assert!(!is_safe_external_url("file:///Users/example/.ssh/id_rsa"));
        assert!(!is_safe_external_url("https://user:pass@example.org/"));
        assert!(!is_safe_external_url("mailto:not-an-email"));
        assert!(!is_safe_external_url("matrix:"));
        assert!(!desktop_url::is_safe_agent_url("https://10.0.0.5/run"));
        assert!(desktop_url::is_safe_agent_url(
            "https://agent.example.org/run"
        ));
    }

    #[test]
    fn windows_secret_store_status_mapping_is_explicit_and_non_persistent() {
        let status = unavailable_secret_store_status(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED);

        assert!(!status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert!(!status.can_persist_session);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED)
        );
        assert!(!bridge_supports_secure_secret_store(&status));
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
}
