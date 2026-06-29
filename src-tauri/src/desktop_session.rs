use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop_url;

pub(crate) const DESKTOP_STORED_SESSION_INVALID: &str = "desktop-stored-session-invalid";

const DESKTOP_SESSION_MAX_BASE_URL_CHARS: usize = 2_048;
const DESKTOP_SESSION_MAX_ID_CHARS: usize = 512;
const DESKTOP_SESSION_MAX_TOKEN_CHARS: usize = 8_192;
const SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS: u64 = 60_000;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSessionEnvelope {
    pub(crate) base_url: String,
    pub(crate) user_id: String,
    pub(crate) device_id: String,
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) expires_in_ms: Option<u64>,
    pub(crate) stored_at_ms: Option<u64>,
}

fn sanitize_required_session_field(
    value: String,
    field_name: &'static str,
    max_chars: usize,
) -> Result<String, String> {
    let sanitized = value.trim().to_owned();
    if sanitized.is_empty() {
        return Err(format!("Session {field_name} cannot be empty"));
    }
    if sanitized.chars().count() > max_chars {
        return Err(format!("Session {field_name} is too long"));
    }
    Ok(sanitized)
}

fn sanitize_optional_session_field(
    value: Option<String>,
    field_name: &'static str,
    max_chars: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let sanitized = value.trim().to_owned();
    if sanitized.is_empty() {
        return Ok(None);
    }
    if sanitized.chars().count() > max_chars {
        return Err(format!("Session {field_name} is too long"));
    }
    Ok(Some(sanitized))
}

pub(crate) fn sanitize_session_envelope(
    session: DesktopSessionEnvelope,
) -> Result<DesktopSessionEnvelope, String> {
    let base_url = sanitize_required_session_field(
        session.base_url,
        "baseUrl",
        DESKTOP_SESSION_MAX_BASE_URL_CHARS,
    )?;
    if !desktop_url::is_allowed_session_base_url(&base_url) {
        return Err(
            "Session baseUrl must be an HTTPS URL or a loopback development URL".to_owned(),
        );
    }

    let user_id =
        sanitize_required_session_field(session.user_id, "userId", DESKTOP_SESSION_MAX_ID_CHARS)?;
    let device_id = sanitize_required_session_field(
        session.device_id,
        "deviceId",
        DESKTOP_SESSION_MAX_ID_CHARS,
    )?;
    let access_token = sanitize_required_session_field(
        session.access_token,
        "accessToken",
        DESKTOP_SESSION_MAX_TOKEN_CHARS,
    )?;
    let refresh_token = sanitize_optional_session_field(
        session.refresh_token,
        "refreshToken",
        DESKTOP_SESSION_MAX_TOKEN_CHARS,
    )?;

    Ok(DesktopSessionEnvelope {
        base_url,
        user_id,
        device_id,
        access_token,
        refresh_token,
        expires_in_ms: session.expires_in_ms,
        stored_at_ms: session.stored_at_ms,
    })
}

pub(crate) fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn session_envelope_is_expired(session: &DesktopSessionEnvelope, now_ms: u64) -> bool {
    let Some(expires_in_ms) = session.expires_in_ms else {
        return false;
    };
    let Some(stored_at_ms) = session.stored_at_ms else {
        return false;
    };

    now_ms
        > stored_at_ms
            .saturating_add(expires_in_ms)
            .saturating_add(SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_session_envelope() -> DesktopSessionEnvelope {
        DesktopSessionEnvelope {
            base_url: "https://matrix.example.org".to_owned(),
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICEID".to_owned(),
            access_token: "access-token".to_owned(),
            refresh_token: None,
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
        }
    }

    #[test]
    fn sanitize_session_envelope_accepts_https_session() {
        let session = sanitize_session_envelope(DesktopSessionEnvelope {
            base_url: " https://matrix.example.org ".to_owned(),
            user_id: " @alice:example.org ".to_owned(),
            device_id: " DEVICEID ".to_owned(),
            access_token: " access-token ".to_owned(),
            refresh_token: Some(" refresh-token ".to_owned()),
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
        })
        .expect("session envelope should pass");

        assert_eq!(session.base_url, "https://matrix.example.org");
        assert_eq!(session.user_id, "@alice:example.org");
        assert_eq!(session.device_id, "DEVICEID");
        assert_eq!(session.access_token, "access-token");
        assert_eq!(session.refresh_token.as_deref(), Some("refresh-token"));
        assert_eq!(session.expires_in_ms, Some(3_600_000));
    }

    #[test]
    fn sanitize_session_envelope_allows_loopback_http_for_development() {
        let mut session = valid_session_envelope();
        session.base_url = "http://localhost:8008".to_owned();

        let sanitized = sanitize_session_envelope(session).expect("loopback session should pass");

        assert_eq!(sanitized.base_url, "http://localhost:8008");
    }

    #[test]
    fn sanitize_session_envelope_rejects_empty_access_token() {
        let mut session = valid_session_envelope();
        session.access_token = "   ".to_owned();

        assert!(sanitize_session_envelope(session).is_err());
    }

    #[test]
    fn sanitize_session_envelope_rejects_plain_http_remote_base_url() {
        let mut session = valid_session_envelope();
        session.base_url = "http://matrix.example.org".to_owned();

        let result = sanitize_session_envelope(session);

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_session_envelope_does_not_echo_token_values_in_errors() {
        let secret_token = "super-secret-access-token";
        let mut session = valid_session_envelope();
        session.base_url = "http://matrix.example.org".to_owned();
        session.access_token = secret_token.to_owned();

        let error = sanitize_session_envelope(session)
            .err()
            .expect("session envelope should fail");

        assert!(!error.contains(secret_token));
    }

    #[test]
    fn session_envelope_expiry_honors_tolerance_and_missing_metadata() {
        let stored_at_ms = 1_000_000;
        let session = DesktopSessionEnvelope {
            stored_at_ms: Some(stored_at_ms),
            expires_in_ms: Some(3_600_000),
            ..valid_session_envelope()
        };

        assert!(!session_envelope_is_expired(&session, stored_at_ms));
        assert!(!session_envelope_is_expired(
            &session,
            stored_at_ms + 3_600_000 + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS
        ));
        assert!(session_envelope_is_expired(
            &session,
            stored_at_ms + 3_600_000 + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS + 1
        ));

        let without_expiry = DesktopSessionEnvelope {
            expires_in_ms: None,
            stored_at_ms: Some(stored_at_ms),
            ..valid_session_envelope()
        };
        assert!(!session_envelope_is_expired(
            &without_expiry,
            stored_at_ms + 9_999_999
        ));

        let without_stored_at = DesktopSessionEnvelope {
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
            ..valid_session_envelope()
        };
        assert!(!session_envelope_is_expired(
            &without_stored_at,
            stored_at_ms + 9_999_999
        ));
    }
}
