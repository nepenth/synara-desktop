//! Privacy redaction helpers for Matrix diagnostics.
//!
//! Tokens, recovery secrets, Matrix user/room/event IDs, homeserver URLs,
//! absolute filesystem paths, raw SDK errors, event bodies, and long opaque
//! secrets must never appear in diagnostic payloads (R0.6 / REV-003). Prefer
//! counts, enums, opaque `diagnostic_id` codes, and temporary correlation
//! tokens owned by the desktop diagnostics layer.

/// Marker substituted for redacted sensitive substrings.
pub const REDACTED: &str = "[redacted]";

/// Maximum length retained for any free-form diagnostic label after sanitization.
pub const MAX_SAFE_LABEL_CHARS: usize = 64;

/// True when `value` looks like a bearer/access/refresh token or similar secret.
pub fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("refresh_token")
        || lower.contains("recovery_key")
        || lower.contains("passphrase")
        || lower.contains("password")
        || lower.contains("secret")
        || lower.contains("private_key")
    {
        return true;
    }
    // Matrix access-token prefixes (common homeserver shapes).
    if value.starts_with("syt_")
        || value.starts_with("srr_")
        || value.starts_with("mct_")
        || value.starts_with("MDAx")
    {
        return true;
    }
    // Long base64-ish blobs are treated as secrets (tokens / keys).
    if value.len() >= 40 && is_mostly_base64ish(value) {
        return true;
    }
    false
}

fn is_mostly_base64ish(value: &str) -> bool {
    let mut ok = 0usize;
    let mut total = 0usize;
    for c in value.chars() {
        total += 1;
        if c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '-' || c == '_' {
            ok += 1;
        }
    }
    total > 0 && ok * 100 / total >= 90
}

/// True when the string looks like a Matrix user / room / event identifier.
pub fn looks_like_matrix_id(value: &str) -> bool {
    let b = value.as_bytes();
    if b.is_empty() {
        return false;
    }
    match b[0] {
        b'@' | b'!' | b'#' | b'$' => value.contains(':') || value.len() > 8,
        _ => false,
    }
}

/// True when the string looks like an absolute URL (homeserver / media / proxy).
pub fn looks_like_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("matrix://")
        || lower.contains("://")
}

/// True when the string embeds URL userinfo credentials (`scheme://user:pass@host`).
pub fn looks_like_url_with_credentials(value: &str) -> bool {
    // After scheme://, userinfo is present when '@' appears before the first path slash
    // of the authority section.
    let Some(scheme_end) = value.find("://") else {
        return false;
    };
    let rest = &value[scheme_end + 3..];
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    authority.contains('@')
}

/// True when the string looks like an absolute filesystem path (Unix or Windows).
pub fn looks_like_absolute_path(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('/') {
        return true;
    }
    // Windows drive paths: C:\... or C:/...
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    // UNC paths
    if trimmed.starts_with("\\\\") {
        return true;
    }
    false
}

/// True when free-form text should be treated as unsafe for diagnostic export.
pub fn looks_like_sensitive_diagnostic(value: &str) -> bool {
    looks_like_secret(value)
        || looks_like_matrix_id(value)
        || looks_like_url(value)
        || looks_like_absolute_path(value)
        || looks_like_url_with_credentials(value)
}

/// Redact known secret patterns and Matrix identifiers from a free-form string.
///
/// Intended for defensive sanitization of untrusted strings before they can be
/// recorded. Preferred path is never accepting free-form secrets at all.
pub fn redact_text(input: &str) -> String {
    if looks_like_sensitive_diagnostic(input) {
        return REDACTED.to_owned();
    }
    // Token / URL / path-ish substrings inside longer messages.
    let lower = input.to_ascii_lowercase();
    for needle in [
        "access_token",
        "refresh_token",
        "recovery_key",
        "bearer ",
        "https://",
        "http://",
        "matrix://",
        "://",
        "file://",
    ] {
        if lower.contains(needle) {
            return REDACTED.to_owned();
        }
    }
    // Absolute path fragments embedded in free-form SDK-style messages.
    if lower.contains("/users/")
        || lower.contains("/home/")
        || lower.contains("/var/")
        || lower.contains("/tmp/")
        || lower.contains("\\users\\")
        || lower.contains("appdata")
        || lower.contains("library/application support")
    {
        return REDACTED.to_owned();
    }
    // Collapse whitespace and bound length for labels.
    let mut out: String = input
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    if out.chars().count() > MAX_SAFE_LABEL_CHARS {
        out = out.chars().take(MAX_SAFE_LABEL_CHARS).collect();
    }
    out
}

/// Keep only a safe diagnostic label: enums, short codes, opaque ids.
///
/// Returns [`None`] when the value cannot be made safe (secrets / ids / URLs / paths).
pub fn safe_diagnostic_label(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if looks_like_sensitive_diagnostic(trimmed) {
        return None;
    }
    // Allow snake_case / kebab / dotted diagnostic codes and short reasons.
    // Disallow path separators so absolute/relative path labels cannot slip through.
    let safe = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    if !safe {
        return None;
    }
    if trimmed.chars().count() > MAX_SAFE_LABEL_CHARS {
        return None;
    }
    Some(trimmed.to_owned())
}

/// Whether a candidate diagnostic field key is known-safe for free attachment.
///
/// Free-form keys that could smuggle secrets (e.g. `accessToken`, `token`) are
/// rejected. The desktop allowlist is the final gate for export.
///
/// Note: counter names such as `recoveryRequests` (sync recovery count) are
/// **allowed** — only secret-bearing recovery/key field names are blocked.
pub fn is_forbidden_field_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    // Credential / secret material field names.
    if lower.contains("access_token")
        || lower.contains("accesstoken")
        || lower.contains("refresh_token")
        || lower.contains("refreshtoken")
        || lower.contains("session_token")
        || lower.contains("sessiontoken")
        || lower == "token"
        || lower.ends_with("_token")
        || lower.ends_with("token")
            && (lower.contains("access")
                || lower.contains("refresh")
                || lower.contains("session")
                || lower.contains("bearer"))
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("secret")
        || lower.contains("passphrase")
        || lower.contains("recovery_key")
        || lower.contains("recoverykey")
        || lower.contains("recovery_secret")
        || lower.contains("recoverysecret")
        || lower.contains("private_key")
        || lower.contains("privatekey")
        || lower.contains("store_key")
        || lower.contains("storekey")
        || lower.contains("store_passphrase")
        || lower.contains("storepassphrase")
        || lower.contains("client_secret")
        || lower.contains("clientsecret")
        || lower.contains("macaroon")
        || lower.contains("authorization")
        || lower.contains("pushkey")
        || lower.contains("push_key")
        || lower.contains("plaintext")
        || lower.contains("event_body")
        || lower.contains("eventbody")
        || lower.contains("decrypted_media")
        || lower.contains("decryptedmedia")
        || lower.contains("media_bytes")
        || lower.contains("mediabytes")
        || lower.contains("raw_push")
        || lower.contains("rawpush")
        || lower.contains("ciphertext")
    {
        return true;
    }
    // Identifier / URL / path field names that must not appear free-form.
    matches!(
        lower.as_str(),
        "userid"
            | "user_id"
            | "roomid"
            | "room_id"
            | "eventid"
            | "event_id"
            | "body"
            | "homeserver"
            | "homeserver_url"
            | "homeserverurl"
            | "baseurl"
            | "base_url"
            | "mxid"
            | "device_id"
            | "deviceid"
            | "wiped_account_root"
            | "wipedaccountroot"
            | "proxy"
            | "proxy_url"
            | "proxyurl"
    ) || lower.ends_with("url")
        || lower.ends_with("_path")
        || lower.ends_with("path") && lower != "filepathkind"
        || lower.contains("absolutepath")
        || lower.contains("absolute_path")
        || lower.contains("accountroot")
        || lower.contains("account_root")
}

#[cfg(test)]
mod redact_unit_tests {
    use super::*;

    #[test]
    fn secrets_and_ids_are_detected() {
        assert!(looks_like_secret("syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345"));
        assert!(looks_like_secret("access_token=abc"));
        assert!(looks_like_matrix_id("@alice:example.org"));
        assert!(looks_like_matrix_id("!room:example.org"));
        assert!(looks_like_matrix_id("$event:example.org"));
        assert!(looks_like_url("https://matrix.example.org"));
        assert!(!looks_like_secret("connectivity"));
        assert!(!looks_like_matrix_id("ready"));
    }

    #[test]
    fn absolute_paths_and_credential_urls_are_detected() {
        assert!(looks_like_absolute_path(
            "/Users/alice/Library/Application Support/Synara"
        ));
        assert!(looks_like_absolute_path(
            "C:\\Users\\alice\\AppData\\Roaming\\Synara"
        ));
        assert!(looks_like_url_with_credentials(
            "https://user:p%40ss@matrix.example.org"
        ));
        assert!(!looks_like_url_with_credentials(
            "https://matrix.example.org"
        ));
        assert!(!looks_like_absolute_path("state"));
        assert!(!looks_like_absolute_path("p2.3-sdk-build-store"));
    }

    #[test]
    fn redact_text_strips_secrets_paths_and_urls() {
        assert_eq!(
            redact_text("syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345"),
            REDACTED
        );
        assert_eq!(redact_text("@bob:hs.example"), REDACTED);
        assert_eq!(redact_text("https://hs.example"), REDACTED);
        assert_eq!(
            redact_text("failed opening /var/app/matrix/acct/state"),
            REDACTED
        );
        assert_eq!(
            redact_text("proxy http://user:secret@127.0.0.1:8080 rejected"),
            REDACTED
        );
        assert_eq!(redact_text("connectivity"), "connectivity");
    }

    #[test]
    fn safe_label_accepts_codes_only() {
        assert_eq!(
            safe_diagnostic_label("p2.5-store-locked"),
            Some("p2.5-store-locked".into())
        );
        assert_eq!(
            safe_diagnostic_label("authentication_rejected"),
            Some("authentication_rejected".into())
        );
        assert_eq!(safe_diagnostic_label("@eve:x"), None);
        assert_eq!(
            safe_diagnostic_label("syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345"),
            None
        );
        assert_eq!(safe_diagnostic_label("https://x"), None);
        assert_eq!(safe_diagnostic_label("/var/app/matrix"), None);
        assert_eq!(safe_diagnostic_label("relative/path"), None);
    }

    #[test]
    fn metric_keys_are_not_false_positive_forbidden() {
        assert!(!is_forbidden_field_key("recoveryRequests"));
        assert!(!is_forbidden_field_key("generation"));
        assert!(!is_forbidden_field_key("queueDepth"));
        assert!(!is_forbidden_field_key("lastFailureCategory"));
        assert!(is_forbidden_field_key("accessToken"));
        assert!(is_forbidden_field_key("refresh_token"));
        assert!(is_forbidden_field_key("recoveryKey"));
        assert!(is_forbidden_field_key("user_id"));
        assert!(is_forbidden_field_key("homeserverUrl"));
        assert!(is_forbidden_field_key("wipedAccountRoot"));
        assert!(is_forbidden_field_key("accountRoot"));
        assert!(is_forbidden_field_key("proxyUrl"));
    }
}
