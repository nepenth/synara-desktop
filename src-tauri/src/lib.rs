#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod build_info;
mod desktop;
mod menu;

use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    LogicalSize, Size, WebviewUrl, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;

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
            desktop::desktop_get_integration_status,
            desktop::desktop_update_tray_state,
            desktop::desktop_get_notification_permission,
            desktop::desktop_request_notification_permission,
            desktop::desktop_notify,
            desktop::desktop_get_performance_capabilities,
            desktop::desktop_agent_action
        ])
        .on_window_event(|window, event| {
            if window.label() != desktop::MAIN_WINDOW_LABEL {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        });

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(desktop::global_shortcut_plugin());
    }

    builder
        .setup(move |app| {
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
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = desktop::show_main_window(app);
            }
        });
}
