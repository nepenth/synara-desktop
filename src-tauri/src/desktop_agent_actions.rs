use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

use crate::desktop_sanitize::{sanitize_action_text, truncate_text};
use crate::desktop_url;

const DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS: usize = 1024;
const DESKTOP_AGENT_ACTION_MAX_URL_CHARS: usize = 2048;
const DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS: usize = 16_384;
const ALLOWED_AGENT_ACTION_KIND: &[&str] = &[
    "agent",
    "copy",
    "continue",
    "export",
    "prompt",
    "regenerate",
    "run",
    "open",
    "open_url",
];

#[derive(Clone, Serialize, serde::Deserialize)]
pub struct DesktopAgentActionPayload {
    id: String,
    title: String,
    kind: Option<String>,
    prompt: Option<String>,
    url: Option<String>,
    markdown: Option<String>,
}

#[derive(Clone, Serialize)]
struct DesktopAgentActionEvent {
    action: DesktopAgentActionPayload,
}

fn sanitize_agent_action_payload(
    mut action: DesktopAgentActionPayload,
) -> Result<DesktopAgentActionPayload, String> {
    action.id = sanitize_action_text(action.id, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
    if action.id.is_empty() {
        return Err("Agent action payload missing action id".to_owned());
    }

    action.title = sanitize_action_text(action.title, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
    if action.title.is_empty() {
        return Err("Agent action payload missing title".to_owned());
    }

    if let Some(kind) = action.kind.take() {
        let normalized = kind.trim().to_lowercase();
        if !ALLOWED_AGENT_ACTION_KIND.contains(&normalized.as_str()) {
            return Err("Agent action kind is not allowed".to_owned());
        }
        action.kind = Some(normalized);
    }

    if let Some(url) = action.url.take() {
        if !desktop_url::is_safe_agent_url(&url) {
            return Err("Agent action URL must use https".to_owned());
        }
        action.url = Some(sanitize_action_text(
            url,
            DESKTOP_AGENT_ACTION_MAX_URL_CHARS,
        ));
    }

    if let Some(prompt) = action.prompt.take() {
        let sanitized = sanitize_action_text(prompt, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
        if !sanitized.is_empty() {
            action.prompt = Some(sanitized);
        }
    }

    if let Some(markdown) = action.markdown.take() {
        let sanitized = truncate_text(markdown, DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS);
        if !sanitized.is_empty() {
            action.markdown = Some(sanitized);
        }
    }

    if action.url.is_none() && action.prompt.is_none() && action.markdown.is_none() {
        return Err("Agent action payload missing runnable payload".to_owned());
    }

    Ok(action)
}

fn extract_agent_action_copy_text(action: &DesktopAgentActionPayload) -> Option<String> {
    if let Some(markdown) = action.markdown.as_deref() {
        return Some(markdown.to_owned());
    }

    if let Some(prompt) = action.prompt.as_deref() {
        return Some(prompt.to_owned());
    }

    if !action.title.is_empty() {
        return Some(action.title.clone());
    }

    None
}

fn handle_agent_action_locally<R: Runtime>(
    app: &AppHandle<R>,
    action: &DesktopAgentActionPayload,
) -> bool {
    match action.kind.as_deref() {
        Some("copy") => {
            let Some(copy_text) = extract_agent_action_copy_text(action) else {
                return false;
            };
            app.clipboard().write_text(copy_text).is_ok()
        }
        Some("open") | Some("open_url") => action
            .url
            .as_ref()
            .is_some_and(|url| app.opener().open_url(url.as_str(), None::<&str>).is_ok()),
        None => action
            .url
            .as_ref()
            .is_some_and(|url| app.opener().open_url(url.as_str(), None::<&str>).is_ok()),
        _ => false,
    }
}

fn is_supported_agent_action(action: &DesktopAgentActionPayload) -> bool {
    match &action.kind {
        Some(kind) => ALLOWED_AGENT_ACTION_KIND.contains(&kind.as_str()),
        None => action.url.is_some() || action.prompt.is_some() || action.markdown.is_some(),
    }
}

#[tauri::command]
pub fn desktop_agent_action<R: Runtime>(
    app: AppHandle<R>,
    action: DesktopAgentActionPayload,
) -> Result<bool, String> {
    let action = sanitize_agent_action_payload(action).map_err(|error| error.to_string())?;
    if !is_supported_agent_action(&action) {
        return Ok(false);
    }

    if handle_agent_action_locally(&app, &action) {
        return Ok(true);
    }

    app.emit("synara://agent-action", DesktopAgentActionEvent { action })
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_action_payload_allows_https_urls() {
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("https://example.org/action".to_owned()),
            markdown: None,
        })
        .expect("action payload should pass");

        assert_eq!(payload.id, "abc");
        assert_eq!(payload.url.as_deref(), Some("https://example.org/action"));
    }

    #[test]
    fn sanitize_action_payload_rejects_plain_http_urls() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("http://example.org/action".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_credentialed_urls() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("https://user:pass@example.org/action".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_disallowed_scheme() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("file:///tmp/test".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_unsupported_kind() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("malicious".to_owned()),
            prompt: Some("Run local tool".to_owned()),
            url: None,
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_requires_payload() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: None,
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn supported_agent_action_detects_no_kind_with_url() {
        let payload = sanitize_action_payload_with_no_kind(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: None,
            prompt: None,
            url: Some("https://example.org/action".to_owned()),
            markdown: None,
        });

        assert!(is_supported_agent_action(&payload));
    }

    #[test]
    fn supported_agent_action_detects_no_kind_with_prompt() {
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: None,
            prompt: Some("Run the workflow".to_owned()),
            url: None,
            markdown: None,
        })
        .expect("prompt-only action should sanitize");

        assert!(is_supported_agent_action(&payload));
    }

    #[test]
    fn sanitize_action_payload_allows_urls_up_to_desktop_max_url_chars() {
        let long_path =
            "a".repeat(DESKTOP_AGENT_ACTION_MAX_URL_CHARS - "https://example.org/".len());
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("open".to_owned()),
            prompt: None,
            url: Some(format!("https://example.org/{long_path}")),
            markdown: None,
        })
        .expect("long https url should sanitize");

        assert_eq!(
            payload.url.as_deref().map(str::len),
            Some(DESKTOP_AGENT_ACTION_MAX_URL_CHARS)
        );
    }

    #[test]
    fn extract_copy_text_prefers_markdown() {
        let payload = sanitize_action_payload_with_no_kind(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("copy".to_owned()),
            prompt: Some("Prompt".to_owned()),
            url: None,
            markdown: Some("```\nBlock\n```".to_owned()),
        });

        assert_eq!(
            extract_agent_action_copy_text(&payload),
            Some("```\nBlock\n```".to_owned())
        );
    }

    fn sanitize_action_payload_with_no_kind(
        action: DesktopAgentActionPayload,
    ) -> DesktopAgentActionPayload {
        sanitize_agent_action_payload(action).expect("action payload should pass")
    }
}
