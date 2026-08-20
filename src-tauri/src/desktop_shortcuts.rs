use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const ROUTE_HOME: &str = "/";
const ROUTE_LATER: &str = "/inbox/later/";
const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";
const ALLOWED_SHORTCUT_LEN: usize = 128;

static LAST_SHORTCUT_APPLY_STATE: OnceLock<Mutex<Option<DesktopShortcutApplyState>>> =
    OnceLock::new();
static LAST_ACTIVE_SHORTCUT_CONFIG: OnceLock<Mutex<Option<DesktopShortcutConfig>>> =
    OnceLock::new();

#[derive(Clone, serde::Deserialize)]
pub struct DesktopShortcutConfig {
    pub show: String,
    pub later: String,
    pub notifications: String,
}

#[derive(Clone, Serialize, serde::Deserialize)]
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
    Unknown,
    Failed,
}

pub(crate) struct DesktopShortcutsIntegrationStatus {
    pub supported: bool,
    pub ready: bool,
    pub message: String,
}

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

fn last_shortcut_state() -> &'static Mutex<Option<DesktopShortcutApplyState>> {
    LAST_SHORTCUT_APPLY_STATE.get_or_init(|| Mutex::new(None))
}

fn set_last_shortcut_apply_state(state: DesktopShortcutApplyState) {
    if let Ok(mut guard) = last_shortcut_state().lock() {
        *guard = Some(state);
    }
}

fn read_last_shortcut_apply_state() -> Option<DesktopShortcutApplyState> {
    last_shortcut_state().lock().ok().and_then(|state| *state)
}

fn last_active_shortcut_config() -> &'static Mutex<Option<DesktopShortcutConfig>> {
    LAST_ACTIVE_SHORTCUT_CONFIG.get_or_init(|| Mutex::new(None))
}

fn read_last_active_shortcut_config() -> Option<DesktopShortcutConfig> {
    last_active_shortcut_config()
        .lock()
        .ok()
        .and_then(|config| config.clone())
}

fn set_last_active_shortcut_config(config: DesktopShortcutConfig) {
    if let Ok(mut guard) = last_active_shortcut_config().lock() {
        *guard = Some(config);
    }
}

fn shortcut_route_for_slot(config: &DesktopShortcutConfig, shortcut: &str) -> Option<&'static str> {
    if config.show == shortcut {
        return Some(ROUTE_HOME);
    }
    if config.later == shortcut {
        return Some(ROUTE_LATER);
    }
    if config.notifications == shortcut {
        return Some(ROUTE_NOTIFICATIONS);
    }
    None
}

fn shortcut_strings_for_config(config: &DesktopShortcutConfig) -> [&str; 3] {
    [
        config.show.as_str(),
        config.later.as_str(),
        config.notifications.as_str(),
    ]
}

fn shortcuts_needing_registration(
    previous: Option<&DesktopShortcutConfig>,
    normalized: &DesktopShortcutConfig,
) -> Vec<String> {
    let Some(previous) = previous else {
        return vec![
            normalized.show.clone(),
            normalized.later.clone(),
            normalized.notifications.clone(),
        ];
    };

    let mut shortcuts = Vec::new();
    if previous.show != normalized.show {
        shortcuts.push(normalized.show.clone());
    }
    if previous.later != normalized.later {
        shortcuts.push(normalized.later.clone());
    }
    if previous.notifications != normalized.notifications {
        shortcuts.push(normalized.notifications.clone());
    }
    shortcuts
}

fn shortcuts_needing_handler_rebind(
    previous: &DesktopShortcutConfig,
    normalized: &DesktopShortcutConfig,
) -> Vec<String> {
    let mut shortcuts = Vec::new();
    for shortcut in shortcut_strings_for_config(normalized) {
        let Some(new_route) = shortcut_route_for_slot(normalized, shortcut) else {
            continue;
        };
        let Some(old_route) = shortcut_route_for_slot(previous, shortcut) else {
            continue;
        };
        if old_route != new_route {
            shortcuts.push(shortcut.to_owned());
        }
    }
    shortcuts
}

fn retired_shortcut_strings(
    previous: &DesktopShortcutConfig,
    normalized: &DesktopShortcutConfig,
) -> Vec<String> {
    let new_strings: HashSet<&str> = shortcut_strings_for_config(normalized)
        .into_iter()
        .collect();
    shortcut_strings_for_config(previous)
        .into_iter()
        .filter(|shortcut| !new_strings.contains(shortcut))
        .map(str::to_owned)
        .collect()
}

fn build_shortcut_route_map(
    _normalized: &DesktopShortcutConfig,
    parsed_show: &Shortcut,
    parsed_later: &Shortcut,
    parsed_notifications: &Shortcut,
) -> HashMap<u32, &'static str> {
    let mut route_by_id = HashMap::new();
    route_by_id.insert(parsed_show.id(), ROUTE_HOME);
    route_by_id.insert(parsed_later.id(), ROUTE_LATER);
    route_by_id.insert(parsed_notifications.id(), ROUTE_NOTIFICATIONS);
    route_by_id
}

fn register_desktop_shortcut_batch(
    global_shortcut: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcuts: &[String],
    route_by_id: HashMap<u32, &'static str>,
) -> Result<(), String> {
    if shortcuts.is_empty() {
        return Ok(());
    }

    let shortcut_refs = shortcuts.iter().map(String::as_str).collect::<Vec<_>>();
    global_shortcut
        .on_shortcuts(shortcut_refs, move |app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let Some(route) = route_by_id.get(&shortcut.id()) else {
                return;
            };
            if let Err(error) = crate::desktop::navigate_main_window(app, route) {
                eprintln!("failed to handle desktop shortcut: {error}");
            }
        })
        .map_err(|error| error.to_string())
}

fn unregister_desktop_shortcut_batch(
    global_shortcut: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcuts: &[String],
) {
    if shortcuts.is_empty() {
        return;
    }

    let shortcut_refs = shortcuts.iter().map(String::as_str).collect::<Vec<_>>();
    let _ = global_shortcut.unregister_multiple(shortcut_refs);
}

fn rebind_desktop_shortcut_handlers(
    global_shortcut: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcuts: &[String],
    route_by_id: HashMap<u32, &'static str>,
) -> Result<(), String> {
    for shortcut in shortcuts {
        let _ = global_shortcut.unregister(shortcut.as_str());
        register_desktop_shortcut_batch(
            global_shortcut,
            std::slice::from_ref(shortcut),
            route_by_id.clone(),
        )?;
    }
    Ok(())
}

fn apply_desktop_shortcuts(
    app: &AppHandle,
    normalized: DesktopShortcutConfig,
    parsed_show: Shortcut,
    parsed_later: Shortcut,
    parsed_notifications: Shortcut,
) -> DesktopShortcutApplyResult {
    let previous_config = read_last_active_shortcut_config();
    let global_shortcut = app.global_shortcut();
    let route_by_id = build_shortcut_route_map(
        &normalized,
        &parsed_show,
        &parsed_later,
        &parsed_notifications,
    );

    let brand_new_shortcuts = shortcuts_needing_registration(previous_config.as_ref(), &normalized);
    if let Err(error) =
        register_desktop_shortcut_batch(global_shortcut, &brand_new_shortcuts, route_by_id.clone())
    {
        unregister_desktop_shortcut_batch(global_shortcut, &brand_new_shortcuts);
        let state = shortcut_state_from_error(&error);
        let preserved_state = read_last_shortcut_apply_state().unwrap_or(state);
        set_last_shortcut_apply_state(preserved_state);
        return shortcut_result(
            state,
            Some(format!("Failed to register desktop shortcuts: {error}")),
            desktop_shortcut_fallback_command(),
        );
    }

    if let Some(previous) = previous_config.as_ref() {
        let rebind_shortcuts = shortcuts_needing_handler_rebind(previous, &normalized);
        if let Err(error) = rebind_desktop_shortcut_handlers(
            global_shortcut,
            &rebind_shortcuts,
            route_by_id.clone(),
        ) {
            unregister_desktop_shortcut_batch(global_shortcut, &brand_new_shortcuts);
            let state = shortcut_state_from_error(&error);
            let preserved_state = read_last_shortcut_apply_state().unwrap_or(state);
            set_last_shortcut_apply_state(preserved_state);
            return shortcut_result(
                state,
                Some(format!("Failed to update desktop shortcuts: {error}")),
                desktop_shortcut_fallback_command(),
            );
        }

        let retired_shortcuts = retired_shortcut_strings(previous, &normalized);
        unregister_desktop_shortcut_batch(global_shortcut, &retired_shortcuts);
    }

    set_last_active_shortcut_config(normalized);
    set_last_shortcut_apply_state(DesktopShortcutApplyState::Active);
    shortcut_result(DesktopShortcutApplyState::Active, None, None)
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

#[cfg(target_os = "linux")]
fn is_gnome_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}

fn shortcut_apply_state_message(state: DesktopShortcutApplyState) -> &'static str {
    match state {
        DesktopShortcutApplyState::Active => "Desktop shortcuts are active.",
        DesktopShortcutApplyState::PermissionNeeded => {
            "Shortcut registration needs permission on this desktop session."
        }
        DesktopShortcutApplyState::Unsupported => {
            "Desktop shortcuts are unsupported in this environment."
        }
        DesktopShortcutApplyState::Unknown => {
            "Desktop shortcut registration has not been attempted yet."
        }
        DesktopShortcutApplyState::Failed => "Desktop shortcut registration failed.",
    }
}

fn shortcut_permission_help_hint() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    {
        Some(
            "On macOS, grant Synara Input Monitoring permission in System Settings > Privacy & Security.",
        )
    }

    #[cfg(target_os = "linux")]
    {
        if is_kde_wayland_session() {
            return Some(
                "On KDE Plasma Wayland, global shortcut capture can require manual registration in System Settings > Shortcuts.",
            );
        }
        if is_wayland() {
            if is_gnome_session() {
                return Some(
                    "On GNOME Wayland, global shortcuts may require portal or compositor permission. Check Settings > Keyboard > Keyboard Shortcuts.",
                );
            }
            return Some(
                "On Wayland sessions, global shortcuts may require portal or compositor permission. Check your desktop environment shortcut settings.",
            );
        }
        if is_kde() {
            return Some(
                "On KDE X11, verify shortcut bindings in System Settings > Shortcuts and ensure no other app has claimed the keys.",
            );
        }
        Some(
            "On Linux X11, verify no other application has claimed the shortcut and check your desktop environment shortcut settings.",
        )
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Some("Check the session permissions for global shortcuts and try again.")
    }
}

fn desktop_shortcut_fallback_command() -> Option<String> {
    if is_kde_wayland_session() {
        return Some(
            "Open System Settings > Shortcuts and create a custom shortcut for Synara.".to_string(),
        );
    }
    None
}

fn unresolved_shortcut_apply_state() -> DesktopShortcutApplyState {
    if is_kde_wayland_session() {
        DesktopShortcutApplyState::Unknown
    } else {
        DesktopShortcutApplyState::Failed
    }
}

fn shortcut_result(
    state: DesktopShortcutApplyState,
    message: Option<String>,
    fallback_command: Option<String>,
) -> DesktopShortcutApplyResult {
    let fallback_command = if matches!(state, DesktopShortcutApplyState::PermissionNeeded) {
        desktop_shortcut_fallback_command()
    } else {
        fallback_command
    };
    let message = message.unwrap_or_else(|| {
        if matches!(state, DesktopShortcutApplyState::PermissionNeeded) {
            let mut parts = vec![shortcut_apply_state_message(state).to_owned()];
            if let Some(hint) = shortcut_permission_help_hint() {
                parts.push(hint.to_owned());
            }
            return parts.join(" ");
        }
        shortcut_apply_state_message(state).to_owned()
    });

    DesktopShortcutApplyResult {
        success: matches!(state, DesktopShortcutApplyState::Active),
        state,
        message,
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

pub(crate) fn desktop_set_shortcuts(
    app: &AppHandle,
    shortcuts: DesktopShortcutConfig,
) -> DesktopShortcutApplyResult {
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

    apply_desktop_shortcuts(
        app,
        normalized,
        parsed_show,
        parsed_later,
        parsed_notifications,
    )
}

pub(crate) fn desktop_shortcuts_integration_status() -> DesktopShortcutsIntegrationStatus {
    let shortcut_state =
        read_last_shortcut_apply_state().unwrap_or_else(unresolved_shortcut_apply_state);
    let message = match shortcut_state {
        DesktopShortcutApplyState::Active => "Global shortcuts are active.".to_string(),
        DesktopShortcutApplyState::PermissionNeeded => {
            let mut parts =
                vec!["Global shortcuts require permission in this desktop session.".to_string()];
            if let Some(hint) = shortcut_permission_help_hint() {
                parts.push(hint.to_owned());
            }
            parts.join(" ")
        }
        DesktopShortcutApplyState::Unsupported => {
            "Global shortcuts are unsupported in this build.".to_string()
        }
        DesktopShortcutApplyState::Unknown => {
            if read_last_active_shortcut_config().is_none() {
                "Global shortcuts are configured after the client loads.".to_string()
            } else {
                "Global shortcut registration has not been attempted yet.".to_string()
            }
        }
        DesktopShortcutApplyState::Failed => {
            if read_last_active_shortcut_config().is_none() {
                "Global shortcuts are configured after the client loads.".to_string()
            } else {
                "Global shortcuts not currently active.".to_string()
            }
        }
    };

    DesktopShortcutsIntegrationStatus {
        supported: cfg!(not(any(target_os = "android", target_os = "ios"))),
        ready: matches!(shortcut_state, DesktopShortcutApplyState::Active),
        message,
    }
}

pub fn global_shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    // Register the plugin without default shortcuts. The command binds them once
    // frontend DesktopShortcutSync mounts. Until then, no shortcuts are active.
    tauri_plugin_global_shortcut::Builder::new().build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
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
    fn shortcut_slot_helpers_detect_rebind_and_retired_shortcuts() {
        let previous = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };
        let swapped = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+L".to_string(),
            later: "CmdOrCtrl+Shift+C".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };

        assert_eq!(
            shortcuts_needing_registration(Some(&previous), &swapped),
            vec![
                "CmdOrCtrl+Shift+L".to_string(),
                "CmdOrCtrl+Shift+C".to_string()
            ]
        );
        assert_eq!(
            shortcuts_needing_handler_rebind(&previous, &swapped),
            vec![
                "CmdOrCtrl+Shift+L".to_string(),
                "CmdOrCtrl+Shift+C".to_string()
            ]
        );
        assert!(retired_shortcut_strings(&previous, &swapped).is_empty());
    }

    #[test]
    fn shortcut_slot_helpers_detect_retired_shortcuts_on_replacement() {
        let previous = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };
        let replaced = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+1".to_string(),
            later: "CmdOrCtrl+Shift+2".to_string(),
            notifications: "CmdOrCtrl+Shift+3".to_string(),
        };

        assert_eq!(
            shortcuts_needing_registration(Some(&previous), &replaced),
            vec![
                "CmdOrCtrl+Shift+1".to_string(),
                "CmdOrCtrl+Shift+2".to_string(),
                "CmdOrCtrl+Shift+3".to_string()
            ]
        );
        assert_eq!(
            retired_shortcut_strings(&previous, &replaced),
            vec![
                "CmdOrCtrl+Shift+C".to_string(),
                "CmdOrCtrl+Shift+L".to_string(),
                "CmdOrCtrl+Shift+N".to_string()
            ]
        );
    }

    #[test]
    fn shortcut_registration_rollback_scope_matches_brand_new_shortcuts() {
        let previous = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };
        let next = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+1".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };

        let brand_new = shortcuts_needing_registration(Some(&previous), &next);
        assert_eq!(brand_new, vec!["CmdOrCtrl+Shift+1".to_string()]);
        assert_eq!(
            retired_shortcut_strings(&previous, &next),
            vec!["CmdOrCtrl+Shift+C".to_string()]
        );
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

        let result = shortcut_result(DesktopShortcutApplyState::PermissionNeeded, None, None);
        assert!(!result.success);
        assert_eq!(result.state, DesktopShortcutApplyState::PermissionNeeded);
        assert!(result.message.contains("permission"));
        assert!(result.message.to_ascii_lowercase().contains("shortcut"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn shortcut_permission_fallback_is_kde_wayland_only() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_wayland = std::env::var("WAYLAND_DISPLAY").ok();

        std::env::remove_var("XDG_CURRENT_DESKTOP");
        std::env::remove_var("WAYLAND_DISPLAY");
        let generic = shortcut_result(DesktopShortcutApplyState::PermissionNeeded, None, None);
        assert!(generic.fallback_command.is_none());
        assert!(!generic
            .message
            .to_ascii_lowercase()
            .contains("kde plasma wayland"));

        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        let kde = shortcut_result(DesktopShortcutApplyState::PermissionNeeded, None, None);
        assert!(kde.fallback_command.is_some());
        assert!(kde
            .message
            .to_ascii_lowercase()
            .contains("kde plasma wayland"));

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
    #[cfg(target_os = "linux")]
    fn unresolved_shortcut_state_is_unknown_on_kde_wayland_before_apply() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_wayland = std::env::var("WAYLAND_DISPLAY").ok();

        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert_eq!(
            unresolved_shortcut_apply_state(),
            DesktopShortcutApplyState::Unknown
        );

        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("XDG_SESSION_TYPE");
        assert_eq!(
            unresolved_shortcut_apply_state(),
            DesktopShortcutApplyState::Failed
        );

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
}
