//! The app side of `desktop_rendering`: the startup call that sets the
//! rendering variables, and the commands behind the "Hardware acceleration"
//! switch in Settings.

use crate::desktop_rendering::{
    decide, nvidia_driver_present, preference_path, read_saved_mode, write_saved_mode,
    RenderingDecision, RenderingMode,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Manager;

/// What this process started with. Unset where the setting does not exist.
static STARTUP: OnceLock<RenderingDecision> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRenderingStatus {
    /// Linux on Wayland only; the other fields mean nothing without it.
    pub available: bool,
    /// The user's choice for the next start; `null` while they have not
    /// chosen and the package's default applies.
    pub saved: Option<RenderingMode>,
    /// The mode this process started with.
    pub active: RenderingMode,
    pub overridden_by_environment: bool,
}

/// Set the rendering variables for this process. Call once, before GTK
/// initialises and while the process has a single thread.
///
/// `identifier` is the app identifier of the compiled Tauri config, so a build
/// made with another `--config` identifier reads its own config directory.
pub fn apply_at_startup(identifier: &str) {
    if !cfg!(target_os = "linux") {
        return;
    }
    let is_wayland = crate::desktop_environment::is_wayland();
    let saved = if is_wayland {
        preference_path(dirs::config_dir(), identifier).and_then(|path| read_saved_mode(&path))
    } else {
        None
    };
    let decision = decide(
        std::env::consts::OS,
        is_wayland,
        saved,
        |key| std::env::var_os(key).map(|value| value.to_string_lossy().into_owned()),
        nvidia_driver_present(),
    );
    for (name, value) in &decision.set {
        // Changing the environment is only sound while no other thread can
        // read it. `run` calls this before it builds the app: the thread
        // `generate_context!` used has been joined, and GTK, the async
        // runtime and every plugin start later.
        std::env::set_var(name, value);
    }
    let _ = STARTUP.set(decision);
}

fn status_with(saved: Option<RenderingMode>) -> DesktopRenderingStatus {
    match STARTUP.get() {
        Some(decision) if decision.applies => DesktopRenderingStatus {
            available: true,
            saved,
            active: decision.mode,
            overridden_by_environment: decision.overridden_by_environment,
        },
        _ => DesktopRenderingStatus {
            available: false,
            saved: None,
            active: RenderingMode::default(),
            overridden_by_environment: false,
        },
    }
}

fn available() -> bool {
    STARTUP.get().is_some_and(|decision| decision.applies)
}

fn preference_file<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(crate::desktop_rendering::PREFERENCE_FILE_NAME))
        .map_err(|error| format!("cannot find the settings directory: {error}"))
}

#[tauri::command]
pub fn desktop_rendering_mode(app: tauri::AppHandle) -> DesktopRenderingStatus {
    if !available() {
        return status_with(None);
    }
    let saved = preference_file(&app)
        .ok()
        .and_then(|path| read_saved_mode(&path));
    status_with(saved)
}

/// Saves the mode for the next start; this process keeps the one it has.
#[tauri::command]
pub fn desktop_set_rendering_mode(
    app: tauri::AppHandle,
    mode: RenderingMode,
) -> Result<DesktopRenderingStatus, String> {
    if !available() {
        return Err("The rendering setting is only available on Linux with Wayland.".to_owned());
    }
    let path = preference_file(&app)?;
    write_saved_mode(&path, mode)
        .map_err(|error| format!("cannot save the rendering setting: {error}"))?;
    Ok(status_with(read_saved_mode(&path)))
}
