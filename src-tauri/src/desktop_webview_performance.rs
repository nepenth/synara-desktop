use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWebviewPerformance {
    pub webview_engine: &'static str,
    pub hardware_acceleration_policy: String,
    pub smooth_scrolling_enabled: Option<bool>,
    pub software_rendering_override_detected: bool,
    pub dmabuf_fast_path_disabled: bool,
}

#[cfg(any(target_os = "linux", test))]
fn environment_flag_with(key: &str, read: impl Fn(&str) -> Option<String>) -> bool {
    read(key).is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    })
}

#[cfg(target_os = "linux")]
fn environment_flag(key: &str) -> bool {
    environment_flag_with(key, |name| std::env::var(name).ok())
}

#[cfg(target_os = "linux")]
fn runtime_state() -> &'static std::sync::RwLock<DesktopWebviewPerformance> {
    use std::sync::{OnceLock, RwLock};

    static STATE: OnceLock<RwLock<DesktopWebviewPerformance>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(DesktopWebviewPerformance {
            webview_engine: "WebKitGTK",
            // WebKitGTK's documented default is on-demand. We intentionally
            // do not force ALWAYS: unsupported hardware/drivers ignore it and
            // Tauri recommends preserving the faster default path unless a
            // specific graphics fault has been demonstrated.
            hardware_acceleration_policy: "on-demand".to_owned(),
            smooth_scrolling_enabled: None,
            software_rendering_override_detected: environment_flag(
                "WEBKIT_DISABLE_COMPOSITING_MODE",
            ) || environment_flag(
                "WEBKIT_SKIA_ENABLE_CPU_RENDERING",
            ),
            dmabuf_fast_path_disabled: environment_flag("WEBKIT_DISABLE_DMABUF_RENDERER"),
        })
    })
}

#[cfg(target_os = "linux")]
pub fn inspect<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    window
        .with_webview(|webview| {
            use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};

            let Some(settings) = webview.inner().settings() else {
                return;
            };

            let policy = match settings.hardware_acceleration_policy() {
                HardwareAccelerationPolicy::Always => "always",
                HardwareAccelerationPolicy::Never => "never",
                HardwareAccelerationPolicy::OnDemand => "on-demand",
                _ => "unknown",
            };

            if let Ok(mut state) = runtime_state().write() {
                state.hardware_acceleration_policy = policy.to_owned();
                state.smooth_scrolling_enabled = Some(settings.enables_smooth_scrolling());
            }
        })
        .map_err(|error| format!("Unable to inspect WebKit performance settings: {error}"))
}

#[cfg(not(target_os = "linux"))]
pub fn inspect<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    Ok(())
}

pub fn capabilities() -> DesktopWebviewPerformance {
    #[cfg(target_os = "linux")]
    {
        return runtime_state()
            .read()
            .map(|state| state.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone());
    }

    #[cfg(target_os = "macos")]
    {
        DesktopWebviewPerformance {
            webview_engine: "WKWebView",
            hardware_acceleration_policy: "system-managed".to_owned(),
            // WKWebView does not expose the WebKitGTK setting. Do not infer a
            // value or override the user's system motion behavior.
            smooth_scrolling_enabled: None,
            software_rendering_override_detected: false,
            dmabuf_fast_path_disabled: false,
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        DesktopWebviewPerformance {
            webview_engine: "WebView2",
            hardware_acceleration_policy: "system-managed".to_owned(),
            smooth_scrolling_enabled: None,
            software_rendering_override_detected: false,
            dmabuf_fast_path_disabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::environment_flag_with;

    #[test]
    fn detects_only_explicit_truthy_rendering_overrides() {
        assert!(environment_flag_with("FLAG", |_| Some("1".to_owned())));
        assert!(environment_flag_with("FLAG", |_| Some(" TRUE ".to_owned())));
        assert!(!environment_flag_with("FLAG", |_| Some("0".to_owned())));
        assert!(!environment_flag_with("FLAG", |_| None));
    }
}
