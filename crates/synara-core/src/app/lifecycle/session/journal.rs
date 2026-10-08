//! Session-lifecycle journal: `logs/matrix-session-lifecycle.log` under the
//! app-data root.
//!
//! Always-available native evidence for token rotation. Only a timestamp and a
//! fixed outcome are written; no account identifiers, tokens, SDK error text or
//! event data are accepted.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use crate::app::lifecycle::LifecycleError;

/// Default id for a rotation save that failed outside a known keyring error.
pub const ROTATION_PERSIST_FAILED_DIAGNOSTIC_ID: &str = "d0.1-session-rotation-persist-failed";

const KEYRING_DIAGNOSTIC_IDS: &[&str] = &[
    "p3.5-keyring-no-entry",
    "p3.5-keyring-encoding",
    "p3.5-keyring-invalid",
    "p3.5-keyring-ambiguous",
    "p3.5-keyring-no-storage-access",
    "p3.5-keyring-platform-failure",
    "p3.5-keyring-unavailable",
    "p3.5-keyring-unsupported-platform",
];

/// Closed id for a vault failure during a rotation save: a known keyring id
/// passes through, anything else collapses to the generic persist failure.
pub fn rotation_persist_diagnostic(error: &LifecycleError) -> &'static str {
    match error {
        LifecycleError::Vault { diagnostic_id, .. }
            if KEYRING_DIAGNOSTIC_IDS.contains(diagnostic_id) =>
        {
            diagnostic_id
        }
        _ => ROTATION_PERSIST_FAILED_DIAGNOSTIC_ID,
    }
}

/// True for the closed ids a failed save may append to the journal.
pub fn is_journal_diagnostic(diagnostic_id: &str) -> bool {
    matches!(
        diagnostic_id,
        "d0.1-session-rotation-persist-failed"
            | "d0.1-session-persistence-retired"
            | "d0.1-session-locator-mismatch"
            | "d0.1-session-locator-sync-failed"
            | "d0.1-session-locator-directory-sync-failed"
    ) || KEYRING_DIAGNOSTIC_IDS.contains(&diagnostic_id)
}

/// Append one rotation outcome. `failure` is the diagnostic id of a failed
/// save, or `None` on success; ids outside the closed set are not written.
pub fn record_session_rotation_outcome(root: &Path, failure: Option<&str>) {
    let outcome = if failure.is_none() {
        "session-rotation-persisted"
    } else {
        "session-rotation-persist-failed"
    };
    let directory = root.join("logs");
    if fs::create_dir_all(&directory).is_err() {
        return;
    }
    let path = directory.join("matrix-session-lifecycle.log");
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > 64 * 1024) {
        let _ = fs::rename(&path, directory.join("matrix-session-lifecycle.log.1"));
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |time| time.as_millis());
        let _ = writeln!(file, "[{timestamp}] native {outcome}");
        if let Some(diagnostic_id) = failure.filter(|id| is_journal_diagnostic(id)) {
            let _ = writeln!(file, "[{timestamp}] native {diagnostic_id}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MatrixIpcErrorCategory;

    #[test]
    fn journal_writes_only_closed_ids() {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-journal-{nanos}"));
        record_session_rotation_outcome(&root, None);
        record_session_rotation_outcome(&root, Some("p3.5-keyring-unavailable"));
        record_session_rotation_outcome(&root, Some("@alice:example.org secret"));
        let log = fs::read_to_string(root.join("logs/matrix-session-lifecycle.log")).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(log.matches("session-rotation-persisted").count(), 1);
        assert_eq!(log.matches("session-rotation-persist-failed").count(), 2);
        assert!(log.contains("p3.5-keyring-unavailable"));
        assert!(!log.contains("alice") && !log.contains("secret"));
    }

    #[test]
    fn unknown_vault_ids_collapse_to_the_generic_persist_failure() {
        let known = LifecycleError::Vault {
            diagnostic_id: "p3.5-keyring-no-entry",
            category: MatrixIpcErrorCategory::StoreUnavailable,
        };
        assert_eq!(rotation_persist_diagnostic(&known), "p3.5-keyring-no-entry");
        let other = LifecycleError::InvalidTarget { diagnostic_id: "x" };
        assert_eq!(
            rotation_persist_diagnostic(&other),
            ROTATION_PERSIST_FAILED_DIAGNOSTIC_ID
        );
    }
}
