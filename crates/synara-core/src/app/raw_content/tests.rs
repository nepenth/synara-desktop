//! Unit tests for P5.9 raw-content extraction.

use super::*;
use std::collections::BTreeMap;

fn map(pairs: &[(&str, ContentValue)]) -> BTreeMap<String, ContentValue> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_raw_content_markers(), MATRIX_RAW_CONTENT_MARKER);
}

#[test]
fn extract_allowlisted_and_unknown() {
    let mut ext = RawContentExtractor::new(1);
    let content = map(&[
        ("body", ContentValue::String("hello agent".into())),
        ("msgtype", ContentValue::String("dev.synara.notice".into())),
        ("agent_id", ContentValue::String("a1".into())),
        ("custom_meta", ContentValue::String("x".into())),
        ("tool_name", ContentValue::String("search".into())),
    ]);
    let out = ext.extract("dev.synara.agent.message", content).unwrap();
    assert!(out.is_agent_event());
    assert_eq!(out.get_str("body"), Some("hello agent"));
    assert_eq!(out.get_str("agent_id"), Some("a1"));
    assert_eq!(
        out.unknown.get("custom_meta").map(String::as_str),
        Some("x")
    );
    assert!(!out.fields.contains_key("custom_meta"));

    let back = RawContentExtractor::reassemble(&out);
    assert_eq!(
        back.get("body"),
        Some(&ContentValue::String("hello agent".into()))
    );
    assert_eq!(
        back.get("custom_meta"),
        Some(&ContentValue::String("x".into()))
    );
}

#[test]
fn rejects_forbidden_keys_and_values() {
    let mut ext = RawContentExtractor::new(1);
    let err = ext
        .extract(
            "dev.synara.agent.message",
            map(&[("access_token", ContentValue::String("nope".into()))]),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.9-forbidden-field");

    let err = ext
        .extract(
            "dev.synara.agent.message",
            map(&[("body", ContentValue::String("access_token=abc".into()))]),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.9-forbidden-value");
}

#[test]
fn allowlist_override_and_retire() {
    let mut ext = RawContentExtractor::new(1);
    ext.set_allowlist(["status".into(), "summary".into()])
        .unwrap();
    let out = ext
        .extract(
            "m.room.message",
            map(&[
                ("status", ContentValue::String("ok".into())),
                ("body", ContentValue::String("ignored-as-unknown".into())),
            ]),
        )
        .unwrap();
    assert_eq!(out.get_str("status"), Some("ok"));
    assert!(!out.fields.contains_key("body"));
    assert_eq!(
        out.unknown.get("body").map(String::as_str),
        Some("ignored-as-unknown")
    );

    ext.retire_generation(4);
    assert_eq!(ext.session_generation(), 4);
    assert!(ext.last("m.room.message").is_none());
}

#[test]
fn invalid_event_type_and_key() {
    let mut ext = RawContentExtractor::new(1);
    let err = ext
        .extract("", map(&[("body", ContentValue::String("x".into()))]))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.9-invalid-event-type");
    let err = ext
        .extract(
            "dev.synara.x",
            map(&[("bad key", ContentValue::String("x".into()))]),
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.9-invalid-key");
}
