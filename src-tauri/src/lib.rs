#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod build_info;
mod desktop;
mod desktop_agent_actions;
mod desktop_file_transfer;
mod desktop_sanitize;
mod desktop_secret_store;
mod desktop_session;
mod desktop_session_store;
mod desktop_shortcuts;
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
    let _ = webview.emit("synara-native-file-drop", payload);
}

#[cfg(target_os = "linux")]
fn normalized_spellcheck_language(value: &str) -> Option<String> {
    let language = value
        .trim()
        .split('.')
        .next()
        .unwrap_or_default()
        .split('@')
        .next()
        .unwrap_or_default()
        .replace('-', "_");

    if language.is_empty()
        || language.eq_ignore_ascii_case("C")
        || language.eq_ignore_ascii_case("POSIX")
    {
        return None;
    }

    Some(language)
}

#[cfg(target_os = "linux")]
fn linux_spellcheck_languages() -> Vec<String> {
    let mut languages = Vec::new();
    for key in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        let Some(value) = std::env::var_os(key).and_then(|value| value.into_string().ok()) else {
            continue;
        };

        for candidate in value.split(':').filter_map(normalized_spellcheck_language) {
            if !languages.contains(&candidate) {
                languages.push(candidate);
            }
        }
    }

    if languages.is_empty() {
        languages.push("en_US".to_owned());
    }

    languages
}

#[cfg(target_os = "linux")]
fn configure_webview_spellcheck<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    use std::sync::atomic::{AtomicBool, Ordering};

    let languages = linux_spellcheck_languages();
    let spellcheck_configured = std::sync::Arc::new(AtomicBool::new(false));
    let configured_flag = spellcheck_configured.clone();
    let configure_result = window.with_webview(move |webview| {
        use webkit2gtk::{WebContextExt, WebViewExt};

        let Some(context) = webview.inner().context() else {
            return;
        };

        let language_refs = languages.iter().map(String::as_str).collect::<Vec<_>>();
        context.set_spell_checking_languages(&language_refs);
        context.set_spell_checking_enabled(true);
        configured_flag.store(true, Ordering::Relaxed);
    });

    if configure_result.is_err() || !spellcheck_configured.load(Ordering::Relaxed) {
        eprintln!(
            "WebKit spellcheck WebContext unavailable; continuing without spellcheck for window {}",
            window.label()
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_webview_spellcheck<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) {}

const PREFERRED_LOCALHOST_PORT: u16 = 44548;
const LOCALHOST_PORT_FALLBACK_COUNT: u16 = 10;

fn is_localhost_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn select_localhost_port() -> Result<u16, String> {
    for offset in 0..LOCALHOST_PORT_FALLBACK_COUNT {
        let port = PREFERRED_LOCALHOST_PORT.saturating_add(offset);
        if is_localhost_port_available(port) {
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

#[cfg(test)]
mod localhost_port_tests {
    use super::{select_localhost_port, PREFERRED_LOCALHOST_PORT};

    #[test]
    fn select_localhost_port_returns_first_available_port() {
        let port = select_localhost_port().expect("localhost port should be available");
        assert!((PREFERRED_LOCALHOST_PORT..PREFERRED_LOCALHOST_PORT + 10).contains(&port));
    }

    #[test]
    fn select_localhost_port_skips_busy_preferred_port() {
        let listener = match std::net::TcpListener::bind(("127.0.0.1", PREFERRED_LOCALHOST_PORT)) {
            Ok(listener) => Some(listener),
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => None,
            Err(error) => panic!("test listener should bind or find an occupied port: {error}"),
        };
        let port = select_localhost_port().expect("fallback localhost port should be available");
        assert_ne!(port, PREFERRED_LOCALHOST_PORT);
        assert!((PREFERRED_LOCALHOST_PORT + 1..PREFERRED_LOCALHOST_PORT + 10).contains(&port));
        drop(listener);
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
            desktop::desktop_get_integration_status,
            desktop::desktop_update_tray_state,
            desktop::desktop_get_notification_permission,
            desktop::desktop_request_notification_permission,
            desktop::desktop_notify,
            desktop::desktop_open_external_url,
            desktop::desktop_save_file,
            desktop::desktop_save_file_begin,
            desktop::desktop_save_file_chunk,
            desktop::desktop_save_file_end,
            desktop::desktop_save_file_abort,
            desktop::desktop_read_dropped_files,
            desktop::desktop_read_dropped_file_chunk,
            desktop::desktop_read_dropped_file_end,
            desktop::desktop_get_performance_capabilities,
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
                    desktop::reset_drag_drop_session();
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
                    desktop::remember_dropped_paths(paths);
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
                    desktop::clear_dropped_file_allowlist_on_drag_leave();
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
        builder = builder
            .plugin(desktop_shortcuts::global_shortcut_plugin())
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_menu(menu::menu(app.handle())?)?;

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
                "{}\nif (window.__SYNARA_DESKTOP__) {{ window.__SYNARA_DESKTOP__.supportsSecureSecretStore = {}; }}",
                include_str!("desktop_bridge.js"),
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

            configure_webview_spellcheck(&window);

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
