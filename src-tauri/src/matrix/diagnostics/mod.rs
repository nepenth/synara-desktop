//! P2.5 — Diagnostics and health model.
//!
//! Privacy-filtered lifecycle, sync, queue, store, task, and error metrics for
//! the Matrix Rust foundation. Projections are compatible with desktop
//! structured diagnostics (session category + allowlisted fields).
//!
//! **Harness / unit tests only until cutover.** No production login/sync loop,
//! no Tauri Matrix commands, no dual-backend, no automatic upload.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.5-diagnostics-health.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod desktop_compat;
mod health;
mod metrics;
mod redact;

pub use desktop_compat::{
    fields_are_desktop_allowlisted, json_contains_forbidden_content, project_health_to_desktop,
    DesktopCompatibleDiagnostic, DESKTOP_CATEGORY_SESSION, EVENT_MATRIX_CLIENT_HEALTH,
    EVENT_MATRIX_STORE_HEALTH, EVENT_SESSION_LIFECYCLE, EVENT_SYNC_METRICS,
    PROJECTED_BOOLEAN_FIELDS, PROJECTED_LABEL_FIELDS, PROJECTED_NUMBER_FIELDS,
};
pub use health::{
    CategoryCount, ErrorHealth, LifecycleHealth, MatrixHealthSnapshot, QueueHealth, StoreHealth,
    StoreHealthStatus, SyncHealth, SyncPhase, TaskHealth, TaskKindCounts,
    MATRIX_HEALTH_SCHEMA_VERSION,
};
pub use metrics::MatrixMetrics;
pub use redact::{
    is_forbidden_field_key, looks_like_absolute_path, looks_like_matrix_id, looks_like_secret,
    looks_like_sensitive_diagnostic, looks_like_url, looks_like_url_with_credentials, redact_text,
    safe_diagnostic_label, MAX_SAFE_LABEL_CHARS, REDACTED,
};

/// Static marker for link / schema smoke (no network, no Client).
pub const MATRIX_DIAGNOSTICS_MARKER: &str = "matrix-diagnostics-health-p2.5";

/// Touch diagnostics/health paths so the foundation remains linked in non-test builds.
pub fn matrix_diagnostics_markers() -> &'static str {
    let metrics = MatrixMetrics::new();
    let snap = metrics.snapshot();
    let projected = project_health_to_desktop(&snap);
    debug_assert_eq!(snap.schema_version, MATRIX_HEALTH_SCHEMA_VERSION);
    debug_assert_eq!(SyncPhase::ALL.len(), 6);
    debug_assert_eq!(StoreHealthStatus::ALL.len(), 6);
    debug_assert!(!projected.is_empty());
    debug_assert_eq!(MATRIX_DIAGNOSTICS_MARKER, "matrix-diagnostics-health-p2.5");
    debug_assert!(!looks_like_secret("connectivity"));
    MATRIX_DIAGNOSTICS_MARKER
}

#[cfg(test)]
mod tests;
