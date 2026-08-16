//! Exact-target local wipe of per-account Matrix store paths (P2.6).
//!
//! Destroys **one** account's native store tree under the derived
//! `account_root`. Never wipes the Matrix root, app-data root, sibling
//! accounts, or non-Matrix local data.
//!
//! Store open / vault failures must **not** call into this module (plan §8.3).

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::app::store::{
    AccountIdentity, StoreKeyId, StoreKeyVault, StorePathError, StorePaths,
    MATRIX_STORE_ROOT_SEGMENT,
};

use super::LifecycleError;

/// Resolved wipe target for exactly one account under one app-data root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WipeTarget {
    identity: AccountIdentity,
    app_data_root: PathBuf,
    paths: StorePaths,
    matrix_root: PathBuf,
}

impl WipeTarget {
    /// Resolve and validate a wipe target. Performs **no** deletion.
    ///
    /// Requirements:
    /// - `app_data_root` absolute, non-empty, no `..` components
    /// - identity fully validated
    /// - account root is exactly one normal component under `{root}/matrix/`
    pub fn resolve(
        app_data_root: impl AsRef<Path>,
        identity: AccountIdentity,
    ) -> Result<Self, LifecycleError> {
        let app_data_root = app_data_root.as_ref();
        validate_app_data_root(app_data_root)?;

        let paths = StorePaths::derive(app_data_root, &identity).map_err(map_path_err)?;
        let matrix_root = app_data_root.join(MATRIX_STORE_ROOT_SEGMENT);

        let target = Self {
            identity,
            app_data_root: app_data_root.to_path_buf(),
            paths,
            matrix_root,
        };
        assert_exact_account_root(&target)?;
        Ok(target)
    }

    pub fn identity(&self) -> &AccountIdentity {
        &self.identity
    }

    pub fn app_data_root(&self) -> &Path {
        &self.app_data_root
    }

    pub fn matrix_root(&self) -> &Path {
        &self.matrix_root
    }

    pub fn paths(&self) -> &StorePaths {
        &self.paths
    }

    pub fn account_segment(&self) -> &str {
        self.paths.account_segment()
    }

    pub fn account_root(&self) -> &Path {
        self.paths.account_root()
    }
}

/// Bounded wipe target category for privacy-safe reports (R0.6 / REV-003).
pub const WIPE_TARGET_KIND_ACCOUNT_ROOT: &str = "account_root";

/// Privacy-safe wipe report (no absolute paths, URLs, or user IDs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WipeReport {
    pub account_segment: String,
    /// True when account root is absent after the operation (idempotent).
    pub account_root_removed: bool,
    pub store_key_removed: bool,
    /// Bounded wipe target kind — never an absolute filesystem path.
    pub wipe_target_kind: String,
}

/// Assert the resolved account root is a direct child of the matrix root.
pub fn assert_exact_account_root(target: &WipeTarget) -> Result<(), LifecycleError> {
    let root = target.account_root();
    if root == target.app_data_root() || root == target.matrix_root() {
        return Err(LifecycleError::WipeRefused {
            diagnostic_id: "p2.6-refuse-root-wipe",
            reason: "refusing to wipe app-data or matrix root",
        });
    }
    if root.parent() != Some(target.matrix_root()) {
        return Err(LifecycleError::WipeRefused {
            diagnostic_id: "p2.6-refuse-nested-or-sibling-layout",
            reason: "account root must be a direct child of matrix root",
        });
    }
    let file_name = root.file_name().and_then(|s| s.to_str());
    if file_name != Some(target.account_segment()) {
        return Err(LifecycleError::TargetMismatch {
            diagnostic_id: "p2.6-account-segment-name-mismatch",
        });
    }
    Ok(())
}

/// Refuse wipe if `candidate` is not exactly the derived account root.
pub fn assert_path_is_wipe_allowed(
    target: &WipeTarget,
    candidate: &Path,
) -> Result<(), LifecycleError> {
    if candidate.as_os_str().is_empty() {
        return Err(LifecycleError::WipeRefused {
            diagnostic_id: "p2.6-empty-wipe-candidate",
            reason: "empty path",
        });
    }
    for comp in candidate.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(LifecycleError::PathEscapesRoot {
                diagnostic_id: "p2.6-wipe-parent-dir-component",
            });
        }
    }
    if candidate == target.app_data_root() || candidate == target.matrix_root() {
        return Err(LifecycleError::WipeRefused {
            diagnostic_id: "p2.6-refuse-root-wipe",
            reason: "refusing to wipe app-data or matrix root",
        });
    }
    if candidate != target.account_root() {
        return Err(LifecycleError::TargetMismatch {
            diagnostic_id: "p2.6-wipe-path-not-exact-account-root",
        });
    }
    // Symlink account roots are refused (avoid following out of the tree).
    if target.account_root().exists() {
        let meta = fs::symlink_metadata(target.account_root())?;
        if meta.file_type().is_symlink() {
            return Err(LifecycleError::WipeRefused {
                diagnostic_id: "p2.6-refuse-symlink-wipe-target",
                reason: "refusing to wipe symlink account root",
            });
        }
    }
    assert_exact_account_root(target)?;
    Ok(())
}

/// Wipe the exact account store tree. Optionally delete the store encryption key.
///
/// Idempotent when the account root is already absent.
pub fn wipe_account_store<K: StoreKeyVault + ?Sized>(
    target: &WipeTarget,
    key_vault: Option<&K>,
) -> Result<WipeReport, LifecycleError> {
    assert_path_is_wipe_allowed(target, target.account_root())?;

    let account_root = target.account_root();
    if account_root.exists() {
        fs::remove_dir_all(account_root)?;
    }
    if account_root.exists() {
        return Err(LifecycleError::WipeRefused {
            diagnostic_id: "p2.6-wipe-root-still-present",
            reason: "account root still present after remove_dir_all",
        });
    }

    let mut store_key_removed = false;
    if let Some(vault) = key_vault {
        let key_id = StoreKeyId::from_identity(target.identity());
        store_key_removed = vault.delete(&key_id)?;
    }

    Ok(WipeReport {
        account_segment: target.account_segment().to_owned(),
        account_root_removed: true,
        store_key_removed,
        wipe_target_kind: WIPE_TARGET_KIND_ACCOUNT_ROOT.to_owned(),
    })
}

fn validate_app_data_root(root: &Path) -> Result<(), LifecycleError> {
    if root.as_os_str().is_empty() {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p2.6-empty-app-data-root",
        });
    }
    if !root.is_absolute() {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p2.6-relative-app-data-root",
        });
    }
    for comp in root.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(LifecycleError::PathEscapesRoot {
                diagnostic_id: "p2.6-app-data-root-parent-dir",
            });
        }
    }
    Ok(())
}

fn map_path_err(err: StorePathError) -> LifecycleError {
    match err {
        StorePathError::PathEscapesRoot => LifecycleError::PathEscapesRoot {
            diagnostic_id: "p2.6-store-path-escapes-root",
        },
        StorePathError::RelativeAppDataRoot => LifecycleError::InvalidTarget {
            diagnostic_id: "r0.4-relative-app-data-root",
        },
        StorePathError::SymlinkRefused => LifecycleError::WipeRefused {
            diagnostic_id: "r0.4-symlink-refused",
            reason: "managed store path must not be a symlink",
        },
        StorePathError::Io(e) => LifecycleError::Io(e),
    }
}
