#[cfg(any(target_os = "linux", test))]
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
pub fn configure_webview_spellcheck<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
    let languages = linux_spellcheck_languages();
    window
        .with_webview(move |webview| {
            use webkit2gtk::{WebContextExt, WebViewExt};

            let Some(context) = webview.inner().context() else {
                eprintln!("[synara] WebKit spellcheck context is unavailable");
                return;
            };

            let language_refs = languages.iter().map(String::as_str).collect::<Vec<_>>();
            context.set_spell_checking_languages(&language_refs);
            context.set_spell_checking_enabled(true);
        })
        .map_err(|error| format!("Unable to configure WebKit spellcheck: {error}"))
}

#[cfg(target_os = "macos")]
pub fn configure_webview_spellcheck<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
    window
        .with_webview(|webview| unsafe {
            use objc2::runtime::NSObjectProtocol;
            use objc2::{msg_send, sel};
            use objc2_app_kit::NSWindow;

            let ns_window: &NSWindow = &*webview.ns_window().cast();
            let Some(responder) = ns_window.firstResponder() else {
                eprintln!("[synara] AppKit spellcheck activation found no first responder");
                return;
            };

            let selector = sel!(setContinuousSpellCheckingEnabled:);
            if responder.respondsToSelector(selector) {
                let _: () = msg_send![&*responder, setContinuousSpellCheckingEnabled: true];
            } else {
                eprintln!("[synara] AppKit first responder does not support continuous spellcheck");
            }
        })
        .map_err(|error| format!("Unable to configure AppKit spellcheck: {error}"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn configure_webview_spellcheck<R: tauri::Runtime>(
    _window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn desktop_enable_spellcheck<R: tauri::Runtime>(
    window: tauri::WebviewWindow<R>,
) -> Result<(), String> {
    configure_webview_spellcheck(&window)
}

#[cfg(test)]
mod tests {
    use super::normalized_spellcheck_language;

    #[test]
    fn normalizes_desktop_locale_for_spellcheck_dictionary_lookup() {
        assert_eq!(
            normalized_spellcheck_language("en-US.UTF-8@calendar=gregorian"),
            Some("en_US".to_owned())
        );
        assert_eq!(
            normalized_spellcheck_language("fr_CA.UTF-8"),
            Some("fr_CA".to_owned())
        );
    }

    #[test]
    fn ignores_non_language_posix_locales() {
        assert_eq!(normalized_spellcheck_language(""), None);
        assert_eq!(normalized_spellcheck_language("C.UTF-8"), None);
        assert_eq!(normalized_spellcheck_language("POSIX"), None);
    }
}
