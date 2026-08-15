//! Per-account Matrix store path layout (plan §8.3, R0.4 / REV-002).

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::identity::AccountIdentity;

/// Top-level directory under the app data root for all Matrix native stores.
pub const MATRIX_STORE_ROOT_SEGMENT: &str = "matrix";

const STATE_SEGMENT: &str = "state";
const CRYPTO_SEGMENT: &str = "crypto";
const CACHE_SEGMENT: &str = "cache";
const MEDIA_SEGMENT: &str = "media";

/// Errors while deriving or preparing store paths (no secrets).
#[derive(Debug)]
pub enum StorePathError {
    Io(io::Error),
    /// Resolved path escaped the expected root (traversal / collision defense).
    PathEscapesRoot,
    /// App-data root must be absolute (relative roots are rejected).
    RelativeAppDataRoot,
    /// A managed path component is a symlink (refused; R0.4 / REV-002).
    SymlinkRefused,
}

impl std::fmt::Display for StorePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "store path io error: {e}"),
            Self::PathEscapesRoot => write!(f, "store path escapes configured root"),
            Self::RelativeAppDataRoot => write!(f, "app data root must be absolute"),
            Self::SymlinkRefused => write!(f, "managed store path must not be a symlink"),
        }
    }
}

impl std::error::Error for StorePathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for StorePathError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Absolute paths for one account's Matrix native stores.
///
/// Layout under `{app_data_root}/matrix/{account_segment}/`:
/// - `state/` — SDK state store (matrix-sdk 0.18 also keeps crypto under state)
/// - `crypto/` — reserved product-managed crypto sidecar (not the sole SDK crypto dir)
/// - `cache/` — event cache path passed to `sqlite_store_with_cache_path`
/// - `media/` — media cache (product-managed; not always bound by Client builder)
///
/// **Honest mapping (REV-006):** `Client::sqlite_store_with_cache_path(state, cache, …)`
/// uses `state/` for both state and crypto SQLite files. `crypto_dir` and
/// `media_dir` are product layout slots, not independent SDK store roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    account_root: PathBuf,
    state_dir: PathBuf,
    crypto_dir: PathBuf,
    cache_dir: PathBuf,
    media_dir: PathBuf,
    account_segment: String,
}

/// Privacy-safe serializable layout description for diagnostics (R0.6 / REV-003).
///
/// Exposes only the opaque account segment and fixed relative child directory
/// names. Absolute filesystem paths never appear on this type.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreLayout {
    /// Opaque non-user directory segment under `{app_data}/matrix/`.
    pub account_segment: String,
    /// Fixed relative child names under the account root (never absolute).
    pub relative_state_dir: String,
    pub relative_crypto_dir: String,
    pub relative_cache_dir: String,
    pub relative_media_dir: String,
    /// Confirms layout is confined under the product Matrix root policy.
    pub confined_under_matrix_root: bool,
}

/// Whether a new Keychain store key may be created for this account layout.
///
/// This is deliberately conservative: once the account root exists, key
/// creation is forbidden until an existing current or known legacy key has
/// been found. That protects encrypted SQLite data as well as interrupted
/// prior opens whose exact on-disk SDK file set cannot be safely inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKeyCreationPolicy {
    AllowForFreshStore,
    ForbidForExistingStore,
}

impl StorePaths {
    /// Derive paths for `identity` under absolute `app_data_root` without creating dirs.
    pub fn derive(
        app_data_root: &Path,
        identity: &AccountIdentity,
    ) -> Result<Self, StorePathError> {
        if !app_data_root.is_absolute() {
            return Err(StorePathError::RelativeAppDataRoot);
        }
        // Reject `.` / `..` components in the configured root itself.
        if app_data_root
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
        {
            return Err(StorePathError::PathEscapesRoot);
        }

        let segment = identity.account_dir_segment();
        if segment.contains("..") || segment.contains('/') || segment.contains('\\') {
            return Err(StorePathError::PathEscapesRoot);
        }

        let matrix_root = app_data_root.join(MATRIX_STORE_ROOT_SEGMENT);
        let account_root = matrix_root.join(&segment);
        ensure_under_root(&matrix_root, &account_root)?;

        let state_dir = account_root.join(STATE_SEGMENT);
        let crypto_dir = account_root.join(CRYPTO_SEGMENT);
        let cache_dir = account_root.join(CACHE_SEGMENT);
        let media_dir = account_root.join(MEDIA_SEGMENT);

        for child in [&state_dir, &crypto_dir, &cache_dir, &media_dir] {
            ensure_under_root(&account_root, child)?;
        }

        Ok(Self {
            account_root,
            state_dir,
            crypto_dir,
            cache_dir,
            media_dir,
            account_segment: segment,
        })
    }

    pub fn account_segment(&self) -> &str {
        &self.account_segment
    }

    pub fn account_root(&self) -> &Path {
        &self.account_root
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub fn crypto_dir(&self) -> &Path {
        &self.crypto_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn media_dir(&self) -> &Path {
        &self.media_dir
    }

    /// Determine whether a missing Keychain key may be generated.
    ///
    /// Call this before any operation that can create the account layout or
    /// revision manifest. A pre-existing account root is protected even when
    /// it is empty: treating it as fresh could replace the key for an
    /// interrupted or legacy encrypted store. This probe never creates,
    /// modifies, archives, or removes any filesystem entry.
    pub fn key_creation_policy(&self) -> Result<StoreKeyCreationPolicy, StorePathError> {
        let matrix_root = self
            .account_root
            .parent()
            .ok_or(StorePathError::PathEscapesRoot)?;
        if let Some(app_root) = matrix_root.parent() {
            refuse_if_symlink(app_root)?;
        }
        refuse_if_symlink(matrix_root)?;
        refuse_if_symlink(&self.account_root)?;

        match fs::symlink_metadata(&self.account_root) {
            Ok(_) => Ok(StoreKeyCreationPolicy::ForbidForExistingStore),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(StoreKeyCreationPolicy::AllowForFreshStore)
            }
            Err(error) => Err(StorePathError::Io(error)),
        }
    }

    /// Create the directory tree with least-privilege permissions where supported.
    ///
    /// Does **not** delete existing content. Refuses symlink components at every
    /// managed path (R0.4 / REV-002). Never wipe on failure (plan §8.3).
    pub fn ensure_dirs(&self) -> Result<(), StorePathError> {
        // Matrix root and account root must not be pre-existing symlinks.
        if let Some(matrix_root) = self.account_root.parent() {
            refuse_if_symlink(matrix_root)?;
            if let Some(app_root) = matrix_root.parent() {
                refuse_if_symlink(app_root)?;
            }
        }
        refuse_if_symlink(&self.account_root)?;

        for dir in [
            &self.account_root,
            &self.state_dir,
            &self.crypto_dir,
            &self.cache_dir,
            &self.media_dir,
        ] {
            refuse_if_symlink(dir)?;
            fs::create_dir_all(dir)?;
            refuse_if_symlink(dir)?;

            // After creation, require canonical path still under the account root.
            let canon_account = self.account_root.canonicalize()?;
            let canon_dir = dir.canonicalize()?;
            if canon_dir != canon_account {
                ensure_under_root(&canon_account, &canon_dir)?;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(dir, perms)?;
            }
        }
        Ok(())
    }

    /// Product/diagnostic projection — never includes key material or absolute paths.
    pub fn layout(&self) -> StoreLayout {
        StoreLayout {
            account_segment: self.account_segment.clone(),
            relative_state_dir: STATE_SEGMENT.to_owned(),
            relative_crypto_dir: CRYPTO_SEGMENT.to_owned(),
            relative_cache_dir: CACHE_SEGMENT.to_owned(),
            relative_media_dir: MEDIA_SEGMENT.to_owned(),
            confined_under_matrix_root: true,
        }
    }
}

fn ensure_under_root(root: &Path, candidate: &Path) -> Result<(), StorePathError> {
    if candidate == root || candidate.starts_with(root) {
        Ok(())
    } else {
        Err(StorePathError::PathEscapesRoot)
    }
}

fn refuse_if_symlink(path: &Path) -> Result<(), StorePathError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(StorePathError::SymlinkRefused),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(StorePathError::Io(e)),
    }
}
