use tauri::menu::{AboutMetadata, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::build_info;
use crate::desktop;

pub const MENU_CHECK_FOR_UPDATES: &str = "synara_check_for_updates";
pub const CHECK_FOR_UPDATES_EVENT: &str = "synara://check-for-updates";

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if event.id().as_ref() != MENU_CHECK_FOR_UPDATES {
        return;
    }

    if let Err(error) = desktop::show_main_window(app) {
        eprintln!("failed to focus Synara before update check: {error}");
    }

    if let Some(window) = app.get_webview_window(desktop::MAIN_WINDOW_LABEL) {
        if let Err(error) = window.emit(CHECK_FOR_UPDATES_EVENT, ()) {
            eprintln!("failed to request update check: {error}");
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
        .paste()
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
