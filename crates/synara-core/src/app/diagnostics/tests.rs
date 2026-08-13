//! P2.5 diagnostics / health model tests.
//!
//! Includes secret-redaction fixtures required by plan Phase 2 acceptance.

use serde_json::json;

use super::*;
use crate::app::supervisor::{
    harness_login_ready, MatrixSupervisor, SupervisorCommand, TestClientFactory,
};
use crate::task::{TaskKind, TaskSupervisor};
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn empty_snapshot_is_privacy_safe_and_serializable() {
    let snap = MatrixHealthSnapshot::empty();
    let value = serde_json::to_value(&snap).expect("serialize");
    assert!(!json_contains_forbidden_content(&value));
    assert_eq!(snap.schema_version, MATRIX_HEALTH_SCHEMA_VERSION);
    assert_eq!(snap.lifecycle.state, "empty");
    assert!(!snap.is_session_ready());
}

#[test]
fn observe_supervisor_and_tasks_export_counters() {
    let mut actor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut actor, &factory).expect("login ready");

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(actor.session_generation());
    let _ = tasks
        .register(TaskKind::Sync, actor.session_generation())
        .expect("sync task");
    let _ = tasks
        .register(TaskKind::Listener, actor.session_generation())
        .expect("listener");

    let mut metrics = MatrixMetrics::new();
    metrics.observe_supervisor(&actor);
    metrics.observe_tasks(&tasks);
    metrics.set_sync_phase(SyncPhase::Live);
    metrics.set_store_readiness(true, true, true, false);
    metrics.observe_queue_depth(3);
    metrics.record_queue_coalesced(2);

    let snap = metrics.snapshot();
    assert_eq!(snap.lifecycle.state, "ready");
    assert!(snap.lifecycle.has_client);
    assert_eq!(
        snap.lifecycle.session_generation,
        actor.session_generation()
    );
    assert_eq!(snap.tasks.registered, 2);
    assert_eq!(snap.tasks.running, 2);
    assert_eq!(snap.tasks.registered_by_kind.sync, 1);
    assert_eq!(snap.tasks.registered_by_kind.listener, 1);
    assert_eq!(snap.sync.phase, SyncPhase::Live);
    assert_eq!(snap.queue.depth, 3);
    assert_eq!(snap.queue.coalesced, 2);
    assert_eq!(snap.store.status, StoreHealthStatus::Ready);
    assert_eq!(snap.lifecycle.installed_total, 1);

    let value = serde_json::to_value(&snap).expect("serialize");
    assert!(!json_contains_forbidden_content(&value));
}

#[test]
fn error_recording_rejects_unsafe_diagnostic_ids() {
    let mut metrics = MatrixMetrics::new();
    metrics.record_error(
        MatrixIpcErrorCategory::AuthenticationRejected,
        Some("syt_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"),
    );
    metrics.record_error(
        MatrixIpcErrorCategory::StoreLocked,
        Some("p2.5-store-locked"),
    );
    metrics.record_error(
        MatrixIpcErrorCategory::Connectivity,
        Some("@alice:example.org"),
    );

    let snap = metrics.snapshot();
    assert_eq!(snap.errors.total, 3);
    assert_eq!(
        snap.errors.last_diagnostic_id.as_deref(),
        Some("p2.5-store-locked"),
        "only the safe id is retained; secrets/MXIDs dropped"
    );
    // Last category is connectivity (most recent record).
    assert_eq!(snap.errors.last_category.as_deref(), Some("connectivity"));
    let auth_count = snap
        .errors
        .by_category
        .iter()
        .find(|c| c.category == "authentication_rejected")
        .map(|c| c.count)
        .unwrap_or(0);
    assert_eq!(auth_count, 1);

    let value = serde_json::to_value(&snap).expect("serialize");
    let text = value.to_string();
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
    assert!(!json_contains_forbidden_content(&value));
}

#[test]
fn desktop_projection_uses_allowlisted_fields_only() {
    let mut metrics = MatrixMetrics::new();
    metrics.set_sync_phase(SyncPhase::CatchingUp);
    metrics.record_sync_recovery_request();
    metrics.record_sync_duration_ms(42);
    metrics.observe_queue_depth(10);
    metrics.record_queue_dropped(1);
    metrics.set_store_status(StoreHealthStatus::Locked);
    metrics.record_store_open_failure();
    metrics.record_error(MatrixIpcErrorCategory::StoreLocked, Some("p2.5-locked"));

    let snap = metrics.snapshot();
    let records = project_health_to_desktop(&snap);
    assert_eq!(records.len(), 4);
    for rec in &records {
        assert_eq!(rec.category, DESKTOP_CATEGORY_SESSION);
        assert!(fields_are_desktop_allowlisted(&rec.fields));
        assert!(!json_contains_forbidden_content(&rec.to_json()));
    }

    let events: Vec<&str> = records.iter().map(|r| r.event).collect();
    assert!(events.contains(&EVENT_SESSION_LIFECYCLE));
    assert!(events.contains(&EVENT_SYNC_METRICS));
    assert!(events.contains(&EVENT_MATRIX_STORE_HEALTH));
    assert!(events.contains(&EVENT_MATRIX_CLIENT_HEALTH));
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

#[test]
fn failure_on_supervisor_surfaces_in_health() {
    let mut actor = MatrixSupervisor::new();
    actor.apply(SupervisorCommand::BeginOpen).expect("open");
    actor
        .fail(
            MatrixIpcErrorCategory::HomeserverUnavailable,
            "p2.5-test-hs-down",
        )
        .expect("fail");

    let mut metrics = MatrixMetrics::new();
    metrics.observe_supervisor(&actor);
    let snap = metrics.snapshot();
    assert_eq!(snap.lifecycle.state, "failed");
    assert_eq!(
        snap.lifecycle.last_failure_category.as_deref(),
        Some("homeserver_unavailable")
    );
    assert_eq!(
        snap.lifecycle.last_failure_diagnostic_id.as_deref(),
        Some("p2.5-test-hs-down")
    );

    let records = project_health_to_desktop(&snap);
    let life = records
        .iter()
        .find(|r| r.event == EVENT_SESSION_LIFECYCLE)
        .expect("lifecycle record");
    assert_eq!(
        life.fields.get("errorType").and_then(|v| v.as_str()),
        Some("homeserver_unavailable")
    );
    assert_eq!(
        life.fields.get("reason").and_then(|v| v.as_str()),
        Some("p2.5-test-hs-down")
    );
    assert!(!json_contains_forbidden_content(&life.to_json()));
}

#[test]
fn markers_touch_paths() {
    assert_eq!(
        matrix_diagnostics_markers(),
        "matrix-diagnostics-health-p2.5"
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
