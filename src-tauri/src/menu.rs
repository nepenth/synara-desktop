use tauri::menu::{AboutMetadata, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::build_info;
use crate::desktop;
use crate::desktop_logging;

pub const MENU_CHECK_FOR_UPDATES: &str = "synara_check_for_updates";
pub const CHECK_FOR_UPDATES_EVENT: &str = "synara://check-for-updates";

/// Menu items whose action the renderer owns (page zoom, Settings). The
/// webview sees the key first and handles it there; these items cover
/// discoverability and mouse use. Payload: the renderer's action name.
pub const APP_SHORTCUT_EVENT: &str = "synara://app-shortcut";
const MENU_APP_SHORTCUTS: [(&str, &str, &str, &str); 4] = [
    // (menu id, label, accelerator, renderer action)
    ("synara_settings", "Settings…", "CmdOrCtrl+,", "settings"),
    (
        "synara_zoom_reset",
        "Actual Size",
        "CmdOrCtrl+0",
        "zoom-reset",
    ),
    ("synara_zoom_in", "Zoom In", "CmdOrCtrl+=", "zoom-in"),
    ("synara_zoom_out", "Zoom Out", "CmdOrCtrl+-", "zoom-out"),
];

fn app_shortcut_action_for_menu_id(id: &str) -> Option<&'static str> {
    MENU_APP_SHORTCUTS
        .iter()
        .find(|(menu_id, ..)| *menu_id == id)
        .map(|(.., action)| *action)
}

fn app_shortcut_item<R: Runtime>(app: &AppHandle<R>, id: &str) -> tauri::Result<MenuItem<R>> {
    let (menu_id, label, accelerator, _) = MENU_APP_SHORTCUTS
        .iter()
        .find(|(menu_id, ..)| *menu_id == id)
        .copied()
        .expect("menu shortcut id is listed");
    MenuItem::with_id(app, menu_id, label, true, Some(accelerator))
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if let Some(action) = app_shortcut_action_for_menu_id(event.id().as_ref()) {
        if let Err(error) = desktop::show_main_window(app) {
            eprintln!("failed to focus Synara for a menu action: {error}");
        }
        if let Some(window) = app.get_webview_window(desktop::MAIN_WINDOW_LABEL) {
            if let Err(error) = window.emit(APP_SHORTCUT_EVENT, action) {
                eprintln!("failed to send a menu action: {error}");
            }
        }
        return;
    }
    if event.id().as_ref() == MENU_CHECK_FOR_UPDATES {
        if let Err(error) = desktop::show_main_window(app) {
            eprintln!("failed to focus Synara before update check: {error}");
            desktop_logging::append_app_log(
                app,
                "native",
                &format!("failed to focus before update check: {error}"),
            );
        }

        if let Some(window) = app.get_webview_window(desktop::MAIN_WINDOW_LABEL) {
            if let Err(error) = window.emit(CHECK_FOR_UPDATES_EVENT, ()) {
                eprintln!("failed to request update check: {error}");
                desktop_logging::append_app_log(
                    app,
                    "native",
                    &format!("failed to request update check: {error}"),
                );
            }
        }
    }
}

pub fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let check_for_updates = MenuItem::with_id(
        app,
        MENU_CHECK_FOR_UPDATES,
        "Check for Updates...",
        true,
        None::<&str>,
    )?;
    let app_menu = SubmenuBuilder::new(app, "Synara")
        .about(Some(AboutMetadata {
            name: Some("Synara".to_owned()),
            version: Some(build_info::label()),
            ..Default::default()
        }))
        .separator()
        .item(&app_shortcut_item(app, "synara_settings")?)
        .item(&check_for_updates)
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        // This module is installed only on macOS. The predefined AppKit Paste
        // command delivers the original clipboard payload to the focused
        // WKWebView, including HTML; the old custom command re-read only text
        // and irreversibly discarded lists and inline formatting before the
        // composer saw the paste. Linux does not install this application
        // menu, so keyboard paste remains WebKitGTK-owned and does not depend
        // on muda's unsupported Wayland synthetic-key path.
        .paste()
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&app_shortcut_item(app, "synara_zoom_reset")?)
        .item(&app_shortcut_item(app, "synara_zoom_in")?)
        .item(&app_shortcut_item(app, "synara_zoom_out")?)
        .separator()
        .fullscreen() // `.fullscreen()` works instead of `.enter_fullscreen()`
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .close_window()
        .build()?; // no `.zoom()` method directly available

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_shortcut_items_map_to_renderer_actions() {
        assert_eq!(
            app_shortcut_action_for_menu_id("synara_settings"),
            Some("settings")
        );
        assert_eq!(
            app_shortcut_action_for_menu_id("synara_zoom_in"),
            Some("zoom-in")
        );
        assert_eq!(
            app_shortcut_action_for_menu_id("synara_zoom_out"),
            Some("zoom-out")
        );
        assert_eq!(
            app_shortcut_action_for_menu_id("synara_zoom_reset"),
            Some("zoom-reset")
        );
        assert_eq!(
            app_shortcut_action_for_menu_id(MENU_CHECK_FOR_UPDATES),
            None
        );
        let accelerators: Vec<&str> = MENU_APP_SHORTCUTS
            .iter()
            .map(|(_, _, accelerator, _)| *accelerator)
            .collect();
        assert_eq!(
            accelerators,
            ["CmdOrCtrl+,", "CmdOrCtrl+0", "CmdOrCtrl+=", "CmdOrCtrl+-"]
        );
    }
}
