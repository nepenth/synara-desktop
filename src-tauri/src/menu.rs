use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

pub fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let app_menu = SubmenuBuilder::new(app, "Cinny")
        .about(Some(Default::default()))
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
