//! Live HTTP pusher set/delete via `Client::pusher()`.
//!
//! Push keys are device tokens. They stay method arguments and never enter
//! `Core::command` JSON. Failed errors are static and never echo push keys,
//! gateway URLs, tokens, or app ids.

use matrix_sdk::ruma::api::client::push::{Pusher, PusherIds, PusherInit, PusherKind};
use matrix_sdk::ruma::push::{HttpPusherData, PushFormat};
use matrix_sdk::Client;
use serde::Serialize;

/// Spec max for `pushkey` (bytes).
pub const MAX_PUSH_KEY_BYTES: usize = 512;
/// Spec max for `app_id` (bytes).
pub const MAX_APP_ID_BYTES: usize = 64;
const MAX_GATEWAY_URL_BYTES: usize = 2048;
const MAX_DISPLAY_NAME_BYTES: usize = 256;
const MAX_LANG_BYTES: usize = 32;

/// Result of HTTP pusher set/delete. `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixHttpPusherWriteResult {
    pub status: &'static str,
}

fn parse_push_key(push_key: &str) -> Result<String, &'static str> {
    if push_key.is_empty()
        || push_key.len() > MAX_PUSH_KEY_BYTES
        || push_key
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte < 0x20 || byte == 0x7f)
    {
        return Err("v-pusher.invalid-push-key");
    }
    Ok(push_key.to_owned())
}

fn parse_app_id(app_id: &str) -> Result<String, &'static str> {
    let trimmed = app_id.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_APP_ID_BYTES {
        return Err("v-pusher.invalid-app-id");
    }
    Ok(trimmed.to_owned())
}

fn parse_bounded_text(
    value: &str,
    max_bytes: usize,
    diagnostic: &'static str,
) -> Result<String, &'static str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_bytes {
        return Err(diagnostic);
    }
    Ok(trimmed.to_owned())
}

fn parse_gateway_url(gateway_url: &str) -> Result<String, &'static str> {
    let trimmed = gateway_url.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_GATEWAY_URL_BYTES
        || trimmed.contains(['\0', ' ', '\n', '\r', '\t'])
    {
        return Err("v-pusher.invalid-gateway");
    }
    let Ok(parsed) = url::Url::parse(trimmed) else {
        return Err("v-pusher.invalid-gateway");
    };
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("v-pusher.invalid-gateway");
    }
    Ok(trimmed.to_owned())
}

/// Register an HTTP pusher. `append` is false so a matching APNs registration
/// replaces rather than duplicates.
pub async fn register_http_pusher(
    client: &Client,
    push_key: &str,
    app_id: &str,
    gateway_url: &str,
    app_display_name: &str,
    device_display_name: &str,
    lang: &str,
) -> Result<MatrixHttpPusherWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-pusher.no-session")?;
    let push_key = parse_push_key(push_key)?;
    let app_id = parse_app_id(app_id)?;
    let gateway_url = parse_gateway_url(gateway_url)?;
    let app_display_name = parse_bounded_text(
        app_display_name,
        MAX_DISPLAY_NAME_BYTES,
        "v-pusher.invalid-name",
    )?;
    let device_display_name = parse_bounded_text(
        device_display_name,
        MAX_DISPLAY_NAME_BYTES,
        "v-pusher.invalid-name",
    )?;
    let lang = parse_bounded_text(lang, MAX_LANG_BYTES, "v-pusher.invalid-lang")?;

    let mut data = HttpPusherData::new(gateway_url);
    data.format = Some(PushFormat::EventIdOnly);
    let pusher = Pusher::from(PusherInit {
        ids: PusherIds::new(push_key, app_id),
        kind: PusherKind::Http(data),
        app_display_name,
        device_display_name,
        profile_tag: None,
        lang,
    });
    client
        .pusher()
        .set(pusher, false)
        .await
        .map_err(|_| "v-pusher.sdk-failed")?;
    Ok(MatrixHttpPusherWriteResult { status: "ok" })
}

/// Delete an HTTP pusher by push key and app id.
pub async fn delete_http_pusher(
    client: &Client,
    push_key: &str,
    app_id: &str,
) -> Result<MatrixHttpPusherWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-pusher.no-session")?;
    let push_key = parse_push_key(push_key)?;
    let app_id = parse_app_id(app_id)?;
    client
        .pusher()
        .delete(PusherIds::new(push_key, app_id))
        .await
        .map_err(|_| "v-pusher.sdk-failed")?;
    Ok(MatrixHttpPusherWriteResult { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_push_key_is_privacy_safe() {
        let secret = "s9-it-push-key-secret";
        let empty = parse_push_key("").unwrap_err();
        let oversize = parse_push_key(&"x".repeat(MAX_PUSH_KEY_BYTES + 1)).unwrap_err();
        assert_eq!(empty, "v-pusher.invalid-push-key");
        assert_eq!(oversize, "v-pusher.invalid-push-key");
        assert!(!empty.contains(secret));
        assert!(!oversize.contains('x'));
        assert_eq!(parse_push_key(secret).unwrap(), secret);
    }

    #[test]
    fn invalid_gateway_is_privacy_safe_https_only() {
        let secret = "https://push.example.org/_matrix/push/v1/notify";
        let http = parse_gateway_url("http://push.example.org/notify").unwrap_err();
        let creds = parse_gateway_url("https://user:token@push.example.org/notify").unwrap_err();
        let empty = parse_gateway_url("").unwrap_err();
        assert_eq!(http, "v-pusher.invalid-gateway");
        assert_eq!(creds, "v-pusher.invalid-gateway");
        assert_eq!(empty, "v-pusher.invalid-gateway");
        assert!(!http.contains("push.example.org"));
        assert!(!creds.contains("token"));
        assert!(!creds.contains("user"));
        assert_eq!(parse_gateway_url(secret).unwrap(), secret);
    }

    #[test]
    fn invalid_app_id_is_static() {
        let marker = "s9-it-app-id-secret";
        let oversize_value = format!("{marker}{}", "x".repeat(MAX_APP_ID_BYTES + 1));
        let oversize = parse_app_id(&oversize_value).unwrap_err();
        assert_eq!(parse_app_id("").unwrap_err(), "v-pusher.invalid-app-id");
        assert_eq!(oversize, "v-pusher.invalid-app-id");
        assert!(!oversize.contains(marker));
    }
}
