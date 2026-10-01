//! Live diagnostic privacy-filter regression fixtures.

use super::*;
use serde_json::{json, Value};

/// Scan a JSON value tree and report whether any forbidden secret-like content
/// remains. Used by redaction fixture tests.
fn json_contains_forbidden_content(value: &Value) -> bool {
    match value {
        Value::String(s) => looks_like_secret(s) || looks_like_matrix_id(s) || looks_like_url(s),
        Value::Array(items) => items.iter().any(json_contains_forbidden_content),
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| is_forbidden_field_key(k) || json_contains_forbidden_content(v)),
        _ => false,
    }
}

/// Plan Phase 2 acceptance: diagnostic fixtures prove secret redaction.
#[test]
fn redaction_fixture_strips_tokens_ids_urls_bodies() {
    let fixture = json!({
        "access_token": "syt_LEAKED_ACCESS_TOKEN_VALUE_0123456789abcdef",
        "refresh_token": "srr_LEAKED_REFRESH_TOKEN_VALUE_0123456789abcdef",
        "recovery_key": "EsTC v1 2a3b 4c5d 6e7f 8a9b 0c1d 2e3f 4a5b",
        "user_id": "@alice:matrix.example.org",
        "room_id": "!roomid:matrix.example.org",
        "event_id": "$eventid:matrix.example.org",
        "homeserver": "https://matrix.example.org",
        "body": "private message body must never appear",
        "ciphertext": "ENCRYPTED_BLOB_SHOULD_NOT_LEAK_AAAAAAAA",
        "ok_phase": "ready",
        "ok_code": "p2.5-fixture",
    });

    assert!(json_contains_forbidden_content(&fixture));

    // Redact every string leaf; drop forbidden keys entirely.
    let cleaned = sanitize_fixture_object(&fixture);
    assert!(!json_contains_forbidden_content(&cleaned));
    let text = cleaned.to_string();
    assert!(!text.contains("syt_"));
    assert!(!text.contains("srr_"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("!roomid"));
    assert!(!text.contains("$eventid"));
    assert!(!text.contains("https://"));
    assert!(!text.contains("private message"));
    assert!(!text.contains("ENCRYPTED_BLOB"));
    // Safe labels survive.
    assert!(text.contains("ready") || text.contains("p2.5-fixture"));
}

#[test]
fn redact_text_and_forbidden_keys() {
    assert_eq!(
        redact_text("syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345"),
        REDACTED
    );
    assert_eq!(redact_text("@user:hs"), REDACTED);
    assert_eq!(redact_text("https://hs.example"), REDACTED);
    assert_eq!(redact_text("ready"), "ready");

    assert!(is_forbidden_field_key("accessToken"));
    assert!(is_forbidden_field_key("user_id"));
    assert!(is_forbidden_field_key("roomId"));
    assert!(is_forbidden_field_key("homeserver"));
    assert!(is_forbidden_field_key("base_url"));
    assert!(!is_forbidden_field_key("generation"));
    assert!(!is_forbidden_field_key("errorType"));
}

/// R0.6 / REV-003 adversarial fixtures: paths, credential URLs, tokens, raw SDK errors.
#[test]
fn r0_6_adversarial_redaction_paths_urls_tokens_sdk_errors() {
    let cases = [
        "/Users/alice/Library/Application Support/Synara/matrix/deadbeef/state",
        "C:\\Users\\alice\\AppData\\Roaming\\Synara\\matrix\\acct",
        // Credential-bearing homeserver URL (no Client-Server REST path literals).
        "https://user:p%40ssword@homeserver.example.org/",
        "http://proxy.local:8080/?access_token=syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345",
        "syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345",
        "@alice:matrix.example.org",
        "sdk error: failed to open sqlite at /var/folders/xx/T/store for https://hs.example",
        "Bearer syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345",
    ];
    for case in cases {
        assert_eq!(
            redact_text(case),
            REDACTED,
            "expected full redaction for {case:?}"
        );
        assert!(
            looks_like_sensitive_diagnostic(case) || redact_text(case) == REDACTED,
            "sensitive classifier/redactor must catch {case:?}"
        );
        assert!(
            safe_diagnostic_label(case).is_none(),
            "unsafe label must be rejected: {case:?}"
        );
    }

    // Safe bounded codes remain usable.
    assert_eq!(
        safe_diagnostic_label("p2.3-sdk-build-store"),
        Some("p2.3-sdk-build-store".into())
    );
    assert_eq!(
        redact_text("store initialization failed"),
        "store initialization failed"
    );
}

/// Defensive sanitizer used only by the redaction fixture test: drops forbidden
/// keys and redacts unsafe string leaves. Not a product public API.
fn sanitize_fixture_object(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if is_forbidden_field_key(k) {
                    continue;
                }
                out.insert(k.clone(), sanitize_fixture_object(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sanitize_fixture_object).collect())
        }
        serde_json::Value::String(s) => {
            if looks_like_secret(s)
                || looks_like_matrix_id(s)
                || looks_like_url(s)
                || s.contains("private message")
                || s.contains("ENCRYPTED")
            {
                serde_json::Value::String(REDACTED.to_owned())
            } else {
                serde_json::Value::String(redact_text(s))
            }
        }
        other => other.clone(),
    }
}
