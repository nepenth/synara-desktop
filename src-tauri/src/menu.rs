use tauri::menu::{AboutMetadata, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::build_info;
use crate::desktop;
use crate::desktop_logging;

pub const MENU_CHECK_FOR_UPDATES: &str = "synara_check_for_updates";
pub const MENU_PASTE: &str = "synara_paste";
pub const CHECK_FOR_UPDATES_EVENT: &str = "synara://check-for-updates";
pub const PASTE_EVENT: &str = "synara://native-paste";

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_CHECK_FOR_UPDATES => {
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
        MENU_PASTE => {
            if let Some(window) = app.get_webview_window(desktop::MAIN_WINDOW_LABEL) {
                if let Err(error) = window.emit(PASTE_EVENT, ()) {
                    eprintln!("failed to request native paste: {error}");
                    desktop_logging::append_app_log(
                        app,
                        "native",
                        &format!("failed to request native paste: {error}"),
                    );
                }

                let script = format!(
                    "window.dispatchEvent(new CustomEvent({}));",
                    serde_json::to_string(PASTE_EVENT)
                        .unwrap_or_else(|_| "\"synara://native-paste\"".to_owned())
                );
                if let Err(error) = window.eval(script) {
                    eprintln!("failed to dispatch native paste event: {error}");
                    desktop_logging::append_app_log(
                        app,
                        "native",
                        &format!("failed to dispatch native paste event: {error}"),
                    );
                }
            }
        }
        _ => {}
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
    let paste = MenuItem::with_id(app, MENU_PASTE, "Paste", true, Some("CmdOrCtrl+V"))?;
    let app_menu = SubmenuBuilder::new(app, "Synara")
        .about(Some(AboutMetadata {
            name: Some("Synara".to_owned()),
            version: Some(build_info::label()),
            ..Default::default()
        }))
        .separator()
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
        .item(&paste)
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
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
