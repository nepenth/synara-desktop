use serde::Serialize;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

pub const MAIN_WINDOW_LABEL: &str = "main";

const MENU_SHOW: &str = "desktop.show";
const MENU_LATER: &str = "desktop.later";
const MENU_NOTIFICATIONS: &str = "desktop.notifications";
const MENU_CHECK_UPDATES: &str = "desktop.check-updates";
const MENU_QUIT: &str = "desktop.quit";

const ROUTE_HOME: &str = "/";
const ROUTE_LATER: &str = "/inbox/later/";
const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";

#[derive(Clone, Serialize)]
struct DesktopActionPayload<'a> {
    action: &'a str,
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

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(
        app,
        MENU_SHOW,
        "Show Cinny",
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
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Cinny", true, Some("CmdOrCtrl+Q"))?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &later,
            &notifications,
            &separator,
            &check_updates,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("cinny-tray")
        .tooltip("Cinny")
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
