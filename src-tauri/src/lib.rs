#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod build_info;
mod desktop;
mod desktop_agent_actions;
mod desktop_file_transfer;
mod desktop_integration;
mod desktop_logging;
mod desktop_notifications;
mod desktop_sanitize;
mod desktop_secret_store;
mod desktop_session;
mod desktop_session_store;
mod desktop_shortcuts;
mod desktop_spellcheck;
mod desktop_tray;
mod desktop_url;
#[cfg(target_os = "macos")]
mod menu;

use serde::Serialize;
use std::path::PathBuf;
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    DragDropEvent, Emitter, LogicalSize, Manager, Size, WebviewUrl, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeFileDropPayload {
    phase: &'static str,
    paths: Vec<String>,
    x: f64,
    y: f64,
}

fn native_drop_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

fn emit_native_file_drop(window: &tauri::Window, payload: NativeFileDropPayload) {
    let Some(webview) = window.get_webview_window(desktop::MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = webview.emit("synara-native-file-drop", payload.clone());
    let Ok(payload_json) = serde_json::to_string(&payload) else {
        return;
    };
    let script = format!(
        "window.dispatchEvent(new CustomEvent('synara-native-file-drop', {{ detail: {} }}));",
        payload_json
    );
    let _ = webview.eval(script);
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn updater_plugin_configured<R: tauri::Runtime>(context: &tauri::Context<R>) -> bool {
    context
        .config()
        .plugins
        .0
        .get("updater")
        .is_some_and(|config| !config.is_null())
}

const PREFERRED_LOCALHOST_PORT: u16 = 44548;
const LOCALHOST_PORT_FALLBACK_COUNT: u16 = 10;

fn is_localhost_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn select_localhost_port_with(mut is_available: impl FnMut(u16) -> bool) -> Result<u16, String> {
    for offset in 0..LOCALHOST_PORT_FALLBACK_COUNT {
        let port = PREFERRED_LOCALHOST_PORT.saturating_add(offset);
        if is_available(port) {
            if offset == 0 {
                eprintln!("[synara] Serving bundled UI on localhost:{port}");
            } else {
                eprintln!(
                    "[synara] Preferred port {PREFERRED_LOCALHOST_PORT} busy; using localhost:{port}"
                );
            }
            return Ok(port);
        }
    }

    Err(format!(
        "No available localhost port in range {PREFERRED_LOCALHOST_PORT}-{}",
        PREFERRED_LOCALHOST_PORT + LOCALHOST_PORT_FALLBACK_COUNT - 1
    ))
}

fn select_localhost_port() -> Result<u16, String> {
    select_localhost_port_with(is_localhost_port_available)
}

#[cfg(test)]
mod localhost_port_tests {
    use super::{select_localhost_port_with, PREFERRED_LOCALHOST_PORT};

    #[test]
    fn select_localhost_port_returns_first_available_port() {
        let port =
            select_localhost_port_with(|_| true).expect("localhost port should be available");
        assert_eq!(port, PREFERRED_LOCALHOST_PORT);
    }

    #[test]
    fn select_localhost_port_skips_busy_preferred_port() {
        let port = select_localhost_port_with(|port| port != PREFERRED_LOCALHOST_PORT)
            .expect("fallback localhost port should be available");
        assert_eq!(port, PREFERRED_LOCALHOST_PORT + 1);
    }
}

pub fn run() {
    let port = match select_localhost_port() {
        Ok(port) => port,
        Err(error) => {
            eprintln!("Failed to start Synara: {error}");
            std::process::exit(1);
        }
    };
    let context = tauri::generate_context!();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let updater_configured = updater_plugin_configured(&context);
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let updater_configured = false;
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_localhost::Builder::new(port).build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            desktop::desktop_show,
            desktop::desktop_hide,
            desktop::desktop_navigate,
            desktop::desktop_set_badge_count,
            desktop::desktop_set_shortcuts,
            desktop::desktop_secret_store_status,
            desktop::desktop_get_session,
            desktop::desktop_set_session,
            desktop::desktop_remove_session,
            desktop_integration::desktop_get_integration_status,
            desktop::desktop_update_tray_state,
            desktop_notifications::desktop_get_notification_permission,
            desktop_notifications::desktop_request_notification_permission,
            desktop_notifications::desktop_notify,
            desktop::desktop_open_external_url,
            desktop_file_transfer::desktop_save_file,
            desktop_file_transfer::desktop_save_file_begin,
            desktop_file_transfer::desktop_save_file_chunk,
            desktop_file_transfer::desktop_save_file_end,
            desktop_file_transfer::desktop_save_file_abort,
            desktop_file_transfer::desktop_read_dropped_files,
            desktop_file_transfer::desktop_read_dropped_file_chunk,
            desktop_file_transfer::desktop_read_dropped_file_end,
            desktop::desktop_get_performance_capabilities,
            desktop_logging::desktop_append_log,
            desktop_logging::desktop_log_path,
            desktop_spellcheck::desktop_enable_spellcheck,
            desktop_agent_actions::desktop_agent_action
        ])
        .on_window_event(|window, event| {
            if window.label() != desktop::MAIN_WINDOW_LABEL {
                return;
            }

            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                WindowEvent::DragDrop(DragDropEvent::Enter { paths, position }) => {
                    desktop_file_transfer::reset_drag_drop_session();
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "enter",
                            paths: native_drop_paths(paths),
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
                WindowEvent::DragDrop(DragDropEvent::Over { position }) => {
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "over",
                            paths: Vec::new(),
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, position }) => {
                    desktop_file_transfer::remember_dropped_paths(paths);
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "drop",
                            paths: native_drop_paths(paths),
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
                WindowEvent::DragDrop(DragDropEvent::Leave) => {
                    desktop_file_transfer::clear_dropped_file_allowlist_on_drag_leave();
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "leave",
                            paths: Vec::new(),
                            x: 0.0,
                            y: 0.0,
                        },
                    );
                }
                _ => {}
            }
        });

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(desktop_shortcuts::global_shortcut_plugin());
        builder = builder.plugin(tauri_plugin_process::init());

        if updater_configured {
            builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
        } else {
            eprintln!(
                "[synara] Tauri updater plugin is disabled because plugins.updater is not configured."
            );
        }
    }

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                app.set_menu(menu::menu(app.handle())?)?;
                app.on_menu_event(menu::handle_menu_event);
            }

            desktop_tray::create_tray(app.handle())?;

            // Dev: use devUrl from tauri.conf.json (http://localhost:8080) to support HMR
            #[cfg(debug_assertions)]
            let window_url = WebviewUrl::App(Default::default());

            // Release: tauri-plugin-localhost serves bundled frontend assets on this port
            #[cfg(not(debug_assertions))]
            let window_url = {
                let localhost_url = format!("http://localhost:{port}");
                let parsed_url = localhost_url
                    .parse::<url::Url>()
                    .map_err(|error| format!("Invalid localhost URL for port {port}: {error}"))?;
                WebviewUrl::External(parsed_url)
            };

            let app_handle = app.handle().clone();
            let bridge_script = format!(
                "{}\nif (window.__SYNARA_DESKTOP__) {{ window.__SYNARA_DESKTOP__.supportsUpdater = {}; window.__SYNARA_DESKTOP__.supportsSecureSecretStore = {}; }}",
                include_str!("desktop_bridge.js"),
                updater_configured,
                desktop::desktop_bridge_supports_secure_secret_store()
            );
            let window = WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Synara")
                .inner_size(1280.0, 900.0)
                .min_inner_size(960.0, 720.0)
                .initialization_script(bridge_script)
                .on_new_window(move |url, _features| {
                    if desktop::is_safe_external_url(url.as_str()) {
                        if let Err(error) = app_handle.opener().open_url(url.as_str(), None::<&str>)
                        {
                            eprintln!("[synara] Failed to open external URL: {error}");
                        }
                    }
                    NewWindowResponse::Deny
                })
                .build()?;

            if let Err(error) = desktop_spellcheck::configure_webview_spellcheck(&window) {
                eprintln!("[synara] {error}");
            }

            if let Ok(size) = window.inner_size() {
                if size.width < 960 || size.height < 720 {
                    window.set_size(Size::Logical(LogicalSize::new(1280.0, 900.0)))?;
                    let _ = window.center();
                }
            }

            window.show()?;
            window.unminimize()?;
            window.set_focus()?;
            Ok(())
        })
        .build(context)
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = desktop::show_main_window(app);
            }

            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
