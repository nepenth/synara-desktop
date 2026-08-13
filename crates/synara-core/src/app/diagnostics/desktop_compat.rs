//! Project Matrix health into desktop-diagnostics-compatible records.
//!
//! Desktop structured diagnostics (`desktop_logging`) accept only:
//! - categories: `session` | `performance` | `room`
//! - event namespaces (session): `matrix-client.`, `matrix-store.`, `sync.`, …
//! - allowlisted numeric / boolean / label field keys
//!
//! This module maps the richer [`MatrixHealthSnapshot`] onto that allowlist so
//! future bridge wiring can call `desktop_record_diagnostic` without expanding
//! the desktop schema prematurely. Full snapshots remain available via serde
//! for harness assertions.

use serde_json::{json, Map, Value};

use super::health::{MatrixHealthSnapshot, StoreHealthStatus, SyncPhase};
use super::redact::{
    is_forbidden_field_key, looks_like_matrix_id, looks_like_secret, looks_like_url,
};

/// Desktop diagnostic category used for Matrix lifecycle health.
pub const DESKTOP_CATEGORY_SESSION: &str = "session";

/// Session-namespace events projected from Matrix health (must stay under
/// `SESSION_EVENT_NAMESPACES` in `desktop_logging.rs`).
pub const EVENT_MATRIX_CLIENT_HEALTH: &str = "matrix-client.health-snapshot";
pub const EVENT_MATRIX_STORE_HEALTH: &str = "matrix-store.health";
pub const EVENT_SYNC_METRICS: &str = "sync.metrics";
pub const EVENT_SESSION_LIFECYCLE: &str = "session.lifecycle";

/// One desktop-compatible diagnostic record (schema-shaped, not yet written).
#[derive(Debug, Clone, PartialEq)]
pub struct DesktopCompatibleDiagnostic {
    pub category: &'static str,
    pub event: &'static str,
    pub fields: Map<String, Value>,
}

impl DesktopCompatibleDiagnostic {
    /// JSON object suitable for tests / future `desktop_record_diagnostic`.
    pub fn to_json(&self) -> Value {
        json!({
            "category": self.category,
            "event": self.event,
            "fields": Value::Object(self.fields.clone()),
        })
    }
}

/// Project a health snapshot into a small set of desktop-compatible records.
pub fn project_health_to_desktop(snap: &MatrixHealthSnapshot) -> Vec<DesktopCompatibleDiagnostic> {
    vec![
        lifecycle_record(snap),
        sync_record(snap),
        store_record(snap),
        tasks_queue_record(snap),
    ]
}

fn lifecycle_record(snap: &MatrixHealthSnapshot) -> DesktopCompatibleDiagnostic {
    let mut fields = Map::new();
    insert_number(&mut fields, "generation", snap.lifecycle.session_generation);
    insert_bool(&mut fields, "hasSession", snap.lifecycle.has_client);
    insert_bool(
        &mut fields,
        "success",
        snap.lifecycle.last_failure_category.is_none(),
    );
    insert_label(&mut fields, "phase", &snap.lifecycle.state);
    insert_label(&mut fields, "status", &snap.lifecycle.state);
    insert_label(&mut fields, "backend", "matrix-rust-sdk");
    if let Some(cat) = &snap.lifecycle.last_failure_category {
        insert_label(&mut fields, "errorType", cat);
    }
    if let Some(id) = &snap.lifecycle.last_failure_diagnostic_id {
        insert_label(&mut fields, "reason", id);
    }
    // Task install/shutdown as bounded counts (map into allowlisted numbers).
    insert_number(&mut fields, "eventCount", snap.lifecycle.installed_total);
    insert_number(&mut fields, "retryCount", snap.lifecycle.shutdown_total);
    DesktopCompatibleDiagnostic {
        category: DESKTOP_CATEGORY_SESSION,
        event: EVENT_SESSION_LIFECYCLE,
        fields,
    }
}

fn sync_record(snap: &MatrixHealthSnapshot) -> DesktopCompatibleDiagnostic {
    let mut fields = Map::new();
    insert_label(&mut fields, "syncState", snap.sync.phase.as_str());
    insert_number(&mut fields, "generation", snap.lifecycle.session_generation);
    insert_number(&mut fields, "eventCount", snap.sync.transition_count);
    insert_number(&mut fields, "retryCount", snap.sync.recovery_requests);
    if let Some(ms) = snap.sync.last_duration_ms {
        insert_number(&mut fields, "durationMs", ms);
    }
    insert_bool(
        &mut fields,
        "success",
        !matches!(snap.sync.phase, SyncPhase::Failed),
    );
    insert_label(&mut fields, "backend", "matrix-rust-sdk");
    DesktopCompatibleDiagnostic {
        category: DESKTOP_CATEGORY_SESSION,
        event: EVENT_SYNC_METRICS,
        fields,
    }
}

fn store_record(snap: &MatrixHealthSnapshot) -> DesktopCompatibleDiagnostic {
    let mut fields = Map::new();
    insert_label(&mut fields, "status", snap.store.status.as_str());
    insert_bool(
        &mut fields,
        "available",
        snap.store.status == StoreHealthStatus::Ready,
    );
    insert_bool(&mut fields, "success", snap.store.open_failures == 0);
    insert_bool(&mut fields, "nativeStoreConfigured", snap.store.state_ready);
    insert_bool(
        &mut fields,
        "nativeStoreAvailable",
        snap.store.state_ready && snap.store.crypto_ready,
    );
    insert_bool(
        &mut fields,
        "nativeStoreError",
        snap.store.open_failures > 0,
    );
    insert_number(&mut fields, "retryCount", snap.store.open_failures);
    insert_label(&mut fields, "backend", "matrix-rust-sdk");
    insert_label(&mut fields, "persistence", "sqlite");
    DesktopCompatibleDiagnostic {
        category: DESKTOP_CATEGORY_SESSION,
        event: EVENT_MATRIX_STORE_HEALTH,
        fields,
    }
}

fn tasks_queue_record(snap: &MatrixHealthSnapshot) -> DesktopCompatibleDiagnostic {
    let mut fields = Map::new();
    insert_number(&mut fields, "generation", snap.tasks.live_generation);
    insert_number(&mut fields, "queueDepth", snap.queue.depth);
    insert_number(&mut fields, "coalescedCount", snap.queue.coalesced);
    insert_number(&mut fields, "eventCount", snap.tasks.spawned_total);
    insert_number(&mut fields, "retryCount", snap.tasks.cancelled_requests);
    insert_number(&mut fields, "waiterCount", snap.tasks.running);
    insert_number(&mut fields, "totalSize", snap.tasks.registered);
    insert_bool(&mut fields, "success", snap.errors.total == 0);
    if let Some(cat) = &snap.errors.last_category {
        insert_label(&mut fields, "errorType", cat);
    }
    if let Some(id) = &snap.errors.last_diagnostic_id {
        insert_label(&mut fields, "reason", id);
    }
    insert_label(&mut fields, "phase", "tasks");
    insert_label(&mut fields, "backend", "matrix-rust-sdk");
    insert_label(&mut fields, "queueState", queue_state_label(snap));
    DesktopCompatibleDiagnostic {
        category: DESKTOP_CATEGORY_SESSION,
        event: EVENT_MATRIX_CLIENT_HEALTH,
        fields,
    }
}

fn queue_state_label(snap: &MatrixHealthSnapshot) -> &'static str {
    if snap.queue.soft_max > 0 && snap.queue.depth >= snap.queue.soft_max {
        "saturated"
    } else if snap.queue.depth == 0 {
        "empty"
    } else {
        "draining"
    }
}

fn insert_number(fields: &mut Map<String, Value>, key: &str, value: u64) {
    if is_forbidden_field_key(key) {
        return;
    }
    fields.insert(key.to_owned(), Value::Number(value.into()));
}

fn insert_bool(fields: &mut Map<String, Value>, key: &str, value: bool) {
    if is_forbidden_field_key(key) {
        return;
    }
    fields.insert(key.to_owned(), Value::Bool(value));
}

fn insert_label(fields: &mut Map<String, Value>, key: &str, value: &str) {
    if is_forbidden_field_key(key) {
        return;
    }
    if looks_like_secret(value) || looks_like_matrix_id(value) || looks_like_url(value) {
        return;
    }
    // Bound length to desktop-safe labels.
    let trimmed: String = value.chars().take(64).collect();
    if trimmed.is_empty() {
        return;
    }
    fields.insert(key.to_owned(), Value::String(trimmed));
}

/// Scan a JSON value tree and report whether any forbidden secret-like content
/// remains. Used by redaction fixture tests.
pub fn json_contains_forbidden_content(value: &Value) -> bool {
    match value {
        Value::String(s) => looks_like_secret(s) || looks_like_matrix_id(s) || looks_like_url(s),
        Value::Array(items) => items.iter().any(json_contains_forbidden_content),
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| is_forbidden_field_key(k) || json_contains_forbidden_content(v)),
        _ => false,
    }
}

/// Allowed desktop field keys that Matrix health projection may emit.
pub const PROJECTED_NUMBER_FIELDS: &[&str] = &[
    "generation",
    "queueDepth",
    "coalescedCount",
    "eventCount",
    "retryCount",
    "waiterCount",
    "totalSize",
    "durationMs",
];

pub const PROJECTED_BOOLEAN_FIELDS: &[&str] = &[
    "hasSession",
    "success",
    "available",
    "nativeStoreConfigured",
    "nativeStoreAvailable",
    "nativeStoreError",
];

pub const PROJECTED_LABEL_FIELDS: &[&str] = &[
    "phase",
    "status",
    "backend",
    "errorType",
    "reason",
    "syncState",
    "persistence",
    "queueState",
];

/// Validate that every field key in a projected record is on the desktop allowlist.
pub fn fields_are_desktop_allowlisted(fields: &Map<String, Value>) -> bool {
    for (key, value) in fields {
        let ok = match value {
            Value::Number(_) => PROJECTED_NUMBER_FIELDS.contains(&key.as_str()),
            Value::Bool(_) => PROJECTED_BOOLEAN_FIELDS.contains(&key.as_str()),
            Value::String(_) => PROJECTED_LABEL_FIELDS.contains(&key.as_str()),
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    true
}
