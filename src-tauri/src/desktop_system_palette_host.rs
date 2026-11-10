//! The app side of `desktop_system_palette`: the command the renderer calls
//! and the watch that announces a theme change.

use crate::desktop_system_palette::{current_system_palette, provider_available, SystemPalette};
use tauri::Emitter;

pub const SYSTEM_PALETTE_CHANGED_EVENT: &str = "synara://system-palette-changed";
const WATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

#[tauri::command]
pub fn desktop_system_palette() -> Option<SystemPalette> {
    current_system_palette()
}

/// Tell the renderer when the desktop's theme changes. Omarchy swaps a
/// symlink, which a file watch on the old target would miss, so this reads
/// the palette every two seconds and emits only when it differs. Does nothing
/// where no provider applies.
pub fn watch_system_palette<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if !provider_available() {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("synara-system-palette".to_owned())
        .spawn(move || {
            let mut last = current_system_palette();
            loop {
                std::thread::sleep(WATCH_INTERVAL);
                let palette = current_system_palette();
                if palette != last {
                    if let Err(error) = app.emit(SYSTEM_PALETTE_CHANGED_EVENT, &palette) {
                        eprintln!("[synara] Unable to announce the system palette: {error}");
                    }
                    last = palette;
                }
            }
        });
    if spawned.is_err() {
        eprintln!("[synara] Unable to start the system palette watch.");
    }
}
