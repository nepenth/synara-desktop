//! `active-session.json`: the non-secret, durable retry locator for a
//! persisted session, and the logout material cleanup keyed on it.
//!
//! The locator is written before any credential write and removed only after
//! every vault delete succeeded, so a failed cleanup always leaves a target the
//! next launch or watcher tick can finish.

use std::fs;
use std::path::{Path, PathBuf};

use super::{SessionFault, SessionFaultKind, SessionLocator};
use crate::app::lifecycle::{clear_session_material, SessionMaterialVault};
use crate::app::store::AccountIdentity;

/// Directory under the app-data root that holds Matrix state.
pub const MATRIX_DATA_DIR: &str = "matrix";
/// Locator file name inside [`MATRIX_DATA_DIR`].
pub const ACTIVE_SESSION_FILE: &str = "active-session.json";

const LOCATOR_MISSING: SessionFault = SessionFault::new(
    SessionFaultKind::Forbidden,
    "No persisted native Matrix session was found.",
    "d0.1-active-session-missing",
);

/// The account identity a locator names. A malformed locator is invalid input.
pub fn locator_account_identity(locator: &SessionLocator) -> Result<AccountIdentity, SessionFault> {
    AccountIdentity::new(&locator.user_id, &locator.homeserver_url)
        .map_err(|_| SessionFault::invalid_input("d0.1-persisted-identity-invalid"))
}

pub fn active_locator_path(app_data_root: &Path) -> PathBuf {
    app_data_root
        .join(MATRIX_DATA_DIR)
        .join(ACTIVE_SESSION_FILE)
}

pub fn write_active_locator(
    app_data_root: &Path,
    locator: &SessionLocator,
) -> Result<(), SessionFault> {
    let path = active_locator_path(app_data_root);
    let parent = path.parent().ok_or(SessionFault::unavailable(
        "d0.1-active-session-path-invalid",
    ))?;
    fs::create_dir_all(parent)
        .map_err(|_| SessionFault::unavailable("d0.1-active-session-dir-failed"))?;
    let bytes = serde_json::to_vec(locator)
        .map_err(|_| SessionFault::unavailable("d0.1-active-session-encode-failed"))?;
    fs::write(path, bytes)
        .map_err(|_| SessionFault::unavailable("d0.1-active-session-write-failed"))
}

pub fn read_active_locator(app_data_root: &Path) -> Result<SessionLocator, SessionFault> {
    let path = active_locator_path(app_data_root);
    if !path.is_file() {
        return Err(LOCATOR_MISSING);
    }
    let bytes =
        fs::read(path).map_err(|_| SessionFault::unavailable("d0.1-active-session-read-failed"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| SessionFault::unavailable("d0.1-active-session-invalid"))
}

pub fn remove_active_locator(app_data_root: &Path) -> Result<(), SessionFault> {
    match fs::remove_file(active_locator_path(app_data_root)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SessionFault::unavailable(
            "d0.1-active-session-remove-failed",
        )),
    }
}

/// Establish the non-secret retry locator before any logout side effect.
/// Readback and filesystem sync errors leave the live session installed.
/// These are OS filesystem sync requests, not a guarantee against hardware or
/// platform-specific power-loss behavior (including macOS fsync limitations).
pub fn ensure_logout_retry_locator(
    root: &Path,
    locator: &SessionLocator,
) -> Result<(), SessionFault> {
    ensure_logout_retry_locator_with_directory_sync(root, locator, |directory| {
        fs::File::open(directory)?.sync_all()
    })
}

/// Narrow I/O fault seam: production readback and file sync remain mandatory.
/// Tests can fail a directory sync before the logout coordinator begins teardown.
pub fn ensure_logout_retry_locator_with_directory_sync(
    root: &Path,
    locator: &SessionLocator,
    mut sync_directory: impl FnMut(&Path) -> std::io::Result<()>,
) -> Result<(), SessionFault> {
    locator_account_identity(locator)?;
    let path = active_locator_path(root);
    if !path.is_file() {
        write_active_locator(root, locator)?;
    }
    if read_active_locator(root)? != *locator {
        return Err(SessionFault::unavailable("d0.1-session-locator-mismatch"));
    }
    fs::File::open(&path)
        .and_then(|file| file.sync_all())
        .map_err(|_| SessionFault::unavailable("d0.1-session-locator-sync-failed"))?;
    // File fsync alone does not persist the directory entry on Linux. Sync
    // containing directories from the leaf to the root, including each parent
    // link create_dir_all may have added. Repeat the whole chain on retries:
    // directories left by an earlier failed preflight may still be unsynced.
    let canonical_path = fs::canonicalize(&path)
        .map_err(|_| SessionFault::unavailable("d0.1-session-locator-directory-sync-failed"))?;
    if let Some(parent) = canonical_path.parent() {
        for directory in parent.ancestors() {
            sync_directory(directory).map_err(|_| {
                SessionFault::unavailable("d0.1-session-locator-directory-sync-failed")
            })?;
        }
    }
    Ok(())
}

/// The locator file is the retry target for failed credential deletion.
/// Delete it only after every vault operation succeeded. The orphan path also
/// uses this after live and Core owners have been retired.
pub fn clear_native_logout_material<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    locator: &SessionLocator,
    root: &Path,
) -> Result<(), SessionFault> {
    let account = locator_account_identity(locator)?;
    ensure_logout_retry_locator(root, locator)?;
    clear_session_material(vault, &account)
        .map_err(|_| SessionFault::unavailable("d0.1-session-clear-failed"))?;
    remove_active_locator(root)
}

/// Clear whatever persisted material the locator names, if any.
pub fn clear_persisted_logout_material<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    root: &Path,
) -> Result<(), SessionFault> {
    if active_locator_path(root).is_file() {
        let locator = read_active_locator(root)?;
        clear_native_logout_material(vault, &locator, root)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("synara-locator-{tag}-{nanos}"));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn locator() -> SessionLocator {
        SessionLocator {
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICE".to_owned(),
            homeserver_url: "https://matrix.example.org".to_owned(),
        }
    }

    #[test]
    fn locator_round_trips_and_missing_is_forbidden() {
        let root = TempRoot::new("round");
        let missing = read_active_locator(root.path()).unwrap_err();
        assert_eq!(missing.kind, SessionFaultKind::Forbidden);
        assert_eq!(missing.diagnostic_id, "d0.1-active-session-missing");
        ensure_logout_retry_locator(root.path(), &locator()).unwrap();
        assert_eq!(read_active_locator(root.path()).unwrap(), locator());
        remove_active_locator(root.path()).unwrap();
        remove_active_locator(root.path()).unwrap();
        assert!(!active_locator_path(root.path()).exists());
    }

    #[test]
    fn retry_locator_refuses_a_different_account_on_disk() {
        let root = TempRoot::new("mismatch");
        write_active_locator(root.path(), &locator()).unwrap();
        let mut other = locator();
        other.device_id = "OTHER".to_owned();
        let error = ensure_logout_retry_locator(root.path(), &other).unwrap_err();
        assert_eq!(error.diagnostic_id, "d0.1-session-locator-mismatch");
    }

    #[test]
    fn directory_sync_failure_is_a_static_fault() {
        let root = TempRoot::new("sync");
        let error =
            ensure_logout_retry_locator_with_directory_sync(root.path(), &locator(), |_| {
                Err(std::io::Error::other("no sync"))
            })
            .unwrap_err();
        assert_eq!(
            error.diagnostic_id,
            "d0.1-session-locator-directory-sync-failed"
        );
    }
}
