#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod build_info;
mod desktop;
#[cfg(target_os = "macos")]
mod menu;

use serde::Serialize;
use std::path::PathBuf;
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    DragDropEvent, LogicalSize, Manager, Size, WebviewUrl, WindowEvent,
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
    let Ok(detail) = serde_json::to_string(&payload) else {
        return;
    };
    let Some(webview) = window.get_webview_window(desktop::MAIN_WINDOW_LABEL) else {
        return;
    };
    let script = format!(
        "window.dispatchEvent(new CustomEvent('synara-native-file-drop', {{ detail: {detail} }}));"
    );
    let _ = webview.eval(&script);
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
    let languages = linux_spellcheck_languages();
    let _ = window.with_webview(move |webview| {
        use webkit2gtk::{WebContextExt, WebViewExt};

        let Some(context) = webview.inner().context() else {
            return;
        };

        let language_refs = languages.iter().map(String::as_str).collect::<Vec<_>>();
        context.set_spell_checking_languages(&language_refs);
        context.set_spell_checking_enabled(true);
    });
}

#[cfg(not(target_os = "linux"))]
fn configure_webview_spellcheck<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) {}

pub fn run() {
    let port: u16 = 44548;
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
            desktop::desktop_read_dropped_files,
            desktop::desktop_get_performance_capabilities,
            desktop::desktop_agent_action
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
        builder = builder.plugin(desktop::global_shortcut_plugin());
    }

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_menu(menu::menu(app.handle())?)?;

            desktop::create_tray(app.handle())?;

            // Dev: use devUrl from tauri.conf.json (http://localhost:8080) to support HMR
            #[cfg(debug_assertions)]
            let window_url = WebviewUrl::App(Default::default());

            // Release: tauri-plugin-localhost serves bundled frontend assets on this port
            #[cfg(not(debug_assertions))]
            let window_url = {
                let url = format!("http://localhost:{}", port).parse().unwrap();
                WebviewUrl::External(url)
            };

            let app_handle = app.handle().clone();
            let window = WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Synara")
                .inner_size(1280.0, 900.0)
                .min_inner_size(960.0, 720.0)
                .initialization_script(include_str!("desktop_bridge.js"))
                .on_new_window(move |url, _features| {
                    if desktop::is_safe_external_url(url.as_str()) {
                        let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
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
