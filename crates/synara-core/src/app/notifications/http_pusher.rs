//! Live HTTP pusher set/delete via `Client::pusher()`.
//!
//! Push keys are device tokens. They stay method arguments and never enter
//! `Core::command` JSON. Failed errors are static and never echo push keys,
//! gateway URLs, tokens, or app ids.

use matrix_sdk::ruma::api::client::push::{get_pushers, Pusher, PusherIds, PusherInit, PusherKind};
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
const APPEND_NEW_PUSHER: bool = false;

/// Result of HTTP pusher set/delete. `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixHttpPusherWriteResult {
    pub status: &'static str,
}

/// Account-bound owner for HTTP-pusher writes.
///
/// This owner retains the exact authenticated Matrix client selected during
/// session attach. Long-lived platform capabilities clone the owner rather
/// than resolving through Core's mutable current-session slot during token or
/// account rotation.
pub struct NativeHttpPusherOwner {
    client: Client,
    device_id: String,
}

impl NativeHttpPusherOwner {
    pub fn new(client: &Client) -> Result<Self, &'static str> {
        let _ = client.user_id().ok_or("v-pusher.no-session")?;
        let device_id = client.device_id().ok_or("v-pusher.no-session")?.to_string();
        Ok(Self {
            client: client.clone(),
            device_id,
        })
    }

    /// Whether this owner is authenticated as the exact shell session asking
    /// for a bound capability. Identity is compared inside Core and is never
    /// returned or included in errors.
    pub fn owns_session(&self, user_id: &str, device_id: &str, homeserver_url: &str) -> bool {
        self.client
            .user_id()
            .is_some_and(|value| value.as_str() == user_id)
            && self
                .client
                .device_id()
                .is_some_and(|value| value.as_str() == device_id)
            && self.client.homeserver().as_str().trim_end_matches('/')
                == homeserver_url.trim().trim_end_matches('/')
    }

    pub async fn register(
        &self,
        push_key: &str,
        app_id: &str,
        gateway_url: &str,
        app_display_name: &str,
        lang: &str,
    ) -> Result<MatrixHttpPusherWriteResult, &'static str> {
        register_http_pusher(
            &self.client,
            push_key,
            app_id,
            gateway_url,
            app_display_name,
            &self.device_id,
            lang,
        )
        .await
    }

    pub async fn delete(
        &self,
        push_key: &str,
        app_id: &str,
    ) -> Result<MatrixHttpPusherWriteResult, &'static str> {
        delete_http_pusher(&self.client, push_key, app_id).await
    }

    /// Enumerate and delete every pusher owned by this app and Matrix device.
    /// Push keys remain inside Core and are never projected over UniFFI. This
    /// makes logout cleanup independent of whether UIKit has redelivered an
    /// APNs token since process launch.
    pub async fn delete_for_device(
        &self,
        app_id: &str,
        last_push_key: Option<&str>,
    ) -> Result<MatrixHttpPusherWriteResult, &'static str> {
        let app_id = parse_app_id(app_id)?;
        // An optional last-known key is only a secondary match hint. A missing
        // or malformed hint must not disable display-name cleanup.
        let last_push_key = optional_last_push_key(last_push_key);
        let response = self
            .client
            .send(get_pushers::v3::Request::new())
            .await
            .map_err(|_| "v-pusher.sdk-failed")?;
        let matching_ids: Vec<PusherIds> = response
            .pushers
            .into_iter()
            .filter(|pusher| {
                matches_device_pusher(pusher, &app_id, &self.device_id, last_push_key.as_deref())
            })
            .map(|pusher| pusher.ids)
            .collect();
        let mut first_error = None;
        for ids in matching_ids {
            if let Err(diagnostic) = self
                .client
                .pusher()
                .delete(ids)
                .await
                .map_err(|_| "v-pusher.sdk-failed")
            {
                if first_error.is_none() {
                    first_error = Some(diagnostic);
                }
            }
        }
        if let Some(diagnostic) = first_error {
            return Err(diagnostic);
        }
        Ok(MatrixHttpPusherWriteResult { status: "ok" })
    }
}

fn matches_device_pusher(
    pusher: &Pusher,
    app_id: &str,
    device_id: &str,
    last_push_key: Option<&str>,
) -> bool {
    if !matches!(pusher.kind, PusherKind::Http(_)) {
        return false;
    }
    pusher.ids.app_id == app_id
        && (pusher.device_display_name == device_id
            || last_push_key.is_some_and(|key| pusher.ids.pushkey == key))
}

fn optional_last_push_key(last_push_key: Option<&str>) -> Option<String> {
    last_push_key.and_then(|key| parse_push_key(key).ok())
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
    let pusher = build_http_pusher(
        push_key,
        app_id,
        gateway_url,
        app_display_name,
        device_display_name,
        lang,
    )?;
    client
        .pusher()
        .set(pusher, APPEND_NEW_PUSHER)
        .await
        .map_err(|_| "v-pusher.sdk-failed")?;
    Ok(MatrixHttpPusherWriteResult { status: "ok" })
}

fn build_http_pusher(
    push_key: &str,
    app_id: &str,
    gateway_url: &str,
    app_display_name: &str,
    device_display_name: &str,
    lang: &str,
) -> Result<Pusher, &'static str> {
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
    Ok(Pusher::from(PusherInit {
        ids: PusherIds::new(push_key, app_id),
        kind: PusherKind::Http(data),
        app_display_name,
        device_display_name,
        profile_tag: None,
        lang,
    }))
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

    #[test]
    fn registration_contract_is_event_id_only_and_replacing() {
        let pusher = build_http_pusher(
            "opaque-device-key",
            "com.whylandcreative.synara",
            "https://push.example.org/_matrix/push/v1/notify",
            "Synara",
            "Device",
            "en-US",
        )
        .unwrap();
        let encoded = serde_json::to_value(pusher).unwrap();
        let data = encoded["data"].as_object().unwrap();

        assert_eq!(data["format"], "event_id_only");
        assert_eq!(
            data.len(),
            2,
            "HTTP pusher data must remain URL + sparse format only"
        );
        assert!(data.contains_key("url"));
        assert!(data.get("body").is_none());
        assert!(data.get("event_id").is_none());
        assert!(data.get("room_id").is_none());
    }

    #[test]
    fn device_cleanup_matches_only_the_exact_app_and_device() {
        let app_id = "com.whylandcreative.synara";
        let exact = build_http_pusher(
            "device-key",
            app_id,
            "https://push.example.org/_matrix/push/v1/notify",
            "Synara",
            "DEVICE",
            "en-US",
        )
        .unwrap();
        let other_device = build_http_pusher(
            "other-key",
            app_id,
            "https://push.example.org/_matrix/push/v1/notify",
            "Synara",
            "OTHER",
            "en-US",
        )
        .unwrap();
        let other_app = build_http_pusher(
            "device-key",
            "org.example.other",
            "https://push.example.org/_matrix/push/v1/notify",
            "Other",
            "DEVICE",
            "en-US",
        )
        .unwrap();

        assert!(matches_device_pusher(&exact, app_id, "DEVICE", None));
        assert!(!matches_device_pusher(
            &other_device,
            app_id,
            "DEVICE",
            None
        ));
        assert!(!matches_device_pusher(
            &other_app,
            app_id,
            "DEVICE",
            Some("device-key")
        ));
    }

    #[test]
    fn device_cleanup_uses_an_exact_last_known_push_key_when_display_name_drifts() {
        let app_id = "com.whylandcreative.synara";
        let mut drifted = build_http_pusher(
            "known-key",
            app_id,
            "https://push.example.org/_matrix/push/v1/notify",
            "Synara",
            "DEVICE",
            "en-US",
        )
        .unwrap();
        drifted.device_display_name.clear();

        assert!(matches_device_pusher(
            &drifted,
            app_id,
            "DEVICE",
            Some("known-key")
        ));
        assert!(!matches_device_pusher(
            &drifted,
            app_id,
            "DEVICE",
            Some("different-key")
        ));
        assert!(!matches_device_pusher(&drifted, app_id, "DEVICE", None));
    }

    #[test]
    fn device_cleanup_ignores_a_malformed_last_key_and_still_matches_display_name() {
        let app_id = "com.whylandcreative.synara";
        let exact = build_http_pusher(
            "device-key",
            app_id,
            "https://push.example.org/_matrix/push/v1/notify",
            "Synara",
            "DEVICE",
            "en-US",
        )
        .unwrap();

        assert!(optional_last_push_key(Some("")).is_none());
        assert!(optional_last_push_key(Some("known-key")).as_deref() == Some("known-key"));
        assert!(matches_device_pusher(
            &exact,
            app_id,
            "DEVICE",
            optional_last_push_key(Some("")).as_deref()
        ));
    }

    #[test]
    fn device_cleanup_ignores_non_http_pushers_with_the_same_app_and_device() {
        let app_id = "com.whylandcreative.synara";
        let mut email = build_http_pusher(
            "device-key",
            app_id,
            "https://push.example.org/_matrix/push/v1/notify",
            "Synara",
            "DEVICE",
            "en-US",
        )
        .unwrap();
        email.kind = PusherKind::Email(matrix_sdk::ruma::api::client::push::EmailPusherData::new());

        assert!(!matches_device_pusher(&email, app_id, "DEVICE", None));
        assert!(!matches_device_pusher(
            &email,
            app_id,
            "DEVICE",
            Some("device-key")
        ));
    }
}
