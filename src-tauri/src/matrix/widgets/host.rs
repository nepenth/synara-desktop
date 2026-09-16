//! Isolated second webview for experimental widgets.
//!
//! Widget windows are not `"main"`. They cannot invoke `matrix_*` commands.
//! The only IPC they receive is `widget_bridge_post` → `WidgetDriverHandle`.

use synara_core::app::widgets::{is_safe_widget_url, widget_target_origin, WidgetOpenResult};
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    AppHandle, Manager, WebviewUrl,
};
use tauri_plugin_opener::OpenerExt;

use crate::desktop;
use crate::matrix::auth::product::MatrixAuthCommandError;

pub const WIDGET_WINDOW_PREFIX: &str = "widget-";

pub fn widget_window_label(session_id: &str) -> String {
    format!("{WIDGET_WINDOW_PREFIX}{session_id}")
}

pub fn session_id_from_widget_label(label: &str) -> Option<&str> {
    label.strip_prefix(WIDGET_WINDOW_PREFIX)
}

pub fn deliver_to_widget_window(app: &AppHandle, session_id: &str, message: &str) {
    let Ok(encoded) = serde_json::to_string(message) else {
        return;
    };
    let script =
        format!("window.__synaraDeliverToWidget && window.__synaraDeliverToWidget({encoded});");
    if let Some(window) = app.get_webview_window(&widget_window_label(session_id)) {
        let _ = window.eval(&script);
    }
}

pub fn destroy_widget_window(app: &AppHandle, session_id: &str) {
    if let Some(window) = app.get_webview_window(&widget_window_label(session_id)) {
        let _ = window.close();
    }
}

pub fn open_widget_window(
    app: &AppHandle,
    opened: &WidgetOpenResult,
    allow_loopback: bool,
) -> Result<(), MatrixAuthCommandError> {
    if !is_safe_widget_url(&opened.webview_url, allow_loopback) {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "The widget URL is not allowed.",
            "experimental-widgets-url-rejected",
        ));
    }
    let parsed = opened.webview_url.parse::<url::Url>().map_err(|_| {
        MatrixAuthCommandError::new(
            "InvalidRequest",
            "The widget URL is not allowed.",
            "experimental-widgets-url-rejected",
        )
    })?;
    // `postMessage` `event.origin` is scheme + authority with no trailing slash;
    // the script compares against this string verbatim.
    let target_origin = opened.target_origin.trim_end_matches('/').to_owned();
    let label = widget_window_label(&opened.session_id);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.close();
    }
    let opener_app = app.clone();
    let navigation_origin = target_origin.clone();
    let init_script = widget_bridge_script(&opened.session_id, &target_origin);
    let title = if opened.name.trim().is_empty() {
        "Widget".to_owned()
    } else {
        opened.name.clone()
    };
    WebviewWindowBuilder::new(app, label, WebviewUrl::External(parsed))
        .title(title)
        .inner_size(960.0, 720.0)
        .initialization_script(init_script)
        .on_navigation(move |url| widget_navigation_allowed(&navigation_origin, url.as_str()))
        .on_new_window(move |url, _features| {
            if desktop::is_safe_external_url(url.as_str()) {
                let _ = opener_app.opener().open_url(url.as_str(), None::<&str>);
            }
            NewWindowResponse::Deny
        })
        .build()
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "The widget window could not be opened.",
                "experimental-widgets-window-open-failed",
            )
        })?;
    Ok(())
}

fn widget_navigation_allowed(target_origin: &str, candidate: &str) -> bool {
    let Some(origin) = widget_target_origin(candidate) else {
        return false;
    };
    origin == target_origin.trim_end_matches('/')
}

fn widget_bridge_script(session_id: &str, target_origin: &str) -> String {
    let session_id = serde_json::to_string(session_id).unwrap_or_else(|_| "\"\"".to_owned());
    let target_origin = serde_json::to_string(target_origin).unwrap_or_else(|_| "\"\"".to_owned());
    format!(
        r#"(function () {{
  var sessionId = {session_id};
  var targetOrigin = {target_origin};
  var delivered = new Set();
  window.__synaraDeliverToWidget = function (message) {{
    if (typeof message !== "string") return;
    delivered.add(message);
    try {{
      window.postMessage(JSON.parse(message), targetOrigin);
    }} catch (_err) {{}}
  }};
  window.addEventListener("message", function (event) {{
    if (event.origin !== targetOrigin) return;
    var serialized;
    try {{
      serialized = typeof event.data === "string" ? event.data : JSON.stringify(event.data);
    }} catch (_err) {{
      return;
    }}
    if (delivered.delete(serialized)) return;
    var internals = window.__TAURI_INTERNALS__;
    if (!internals || typeof internals.invoke !== "function") return;
    internals.invoke("widget_bridge_post", {{ message: serialized }});
  }});
}})();"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_labels_are_prefixed_and_not_main() {
        assert_eq!(widget_window_label("w1"), "widget-w1");
        assert_eq!(session_id_from_widget_label("widget-w1"), Some("w1"));
        assert_eq!(session_id_from_widget_label("main"), None);
        assert_ne!(widget_window_label("w1"), crate::desktop::MAIN_WINDOW_LABEL);
    }

    #[test]
    fn navigation_stays_on_the_widget_origin() {
        assert!(widget_navigation_allowed(
            "https://widgets.example.org",
            "https://widgets.example.org/app?widgetId=1"
        ));
        assert!(!widget_navigation_allowed(
            "https://widgets.example.org",
            "https://evil.example/app"
        ));
        assert!(!widget_navigation_allowed(
            "https://widgets.example.org",
            "javascript:alert(1)"
        ));
    }

    #[test]
    fn bridge_script_compares_against_a_slashless_origin() {
        let script = widget_bridge_script("w1", "https://widgets.example.org");
        assert!(script.contains("\"https://widgets.example.org\""));
        assert!(!script.contains("\"https://widgets.example.org/\""));
        assert!(script.contains("event.origin !== targetOrigin"));
    }
}
