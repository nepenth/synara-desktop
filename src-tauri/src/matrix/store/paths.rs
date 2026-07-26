//! Per-account Matrix store path layout (plan §8.3).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
}

impl std::fmt::Display for StorePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "store path io error: {e}"),
            Self::PathEscapesRoot => write!(f, "store path escapes configured root"),
        }
    }
}

impl std::error::Error for StorePathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::PathEscapesRoot => None,
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
/// - `state/` — state store
/// - `crypto/` — crypto store
/// - `cache/` — event cache
/// - `media/` — media cache
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    account_root: PathBuf,
    state_dir: PathBuf,
    crypto_dir: PathBuf,
    cache_dir: PathBuf,
    media_dir: PathBuf,
    account_segment: String,
}

/// Serializable layout description for diagnostics (paths only — never keys).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreLayout {
    pub account_segment: String,
    pub account_root: String,
    pub state_dir: String,
    pub crypto_dir: String,
    pub cache_dir: String,
    pub media_dir: String,
}

impl StorePaths {
    /// Derive paths for `identity` under `app_data_root` without creating dirs.
    pub fn derive(
        app_data_root: &Path,
        identity: &AccountIdentity,
    ) -> Result<Self, StorePathError> {
        let segment = identity.account_dir_segment();
        // Reject traversal in the segment itself (fingerprint/sanitize should not
        // produce this; defense in depth).
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

    /// Create the directory tree with least-privilege permissions where supported.
    ///
    /// Does **not** delete existing content. Crash recovery / open failure must
    /// call this (or only open) — never wipe on failure (plan §8.3).
    pub fn ensure_dirs(&self) -> Result<(), StorePathError> {
        for dir in [
            &self.account_root,
            &self.state_dir,
            &self.crypto_dir,
            &self.cache_dir,
            &self.media_dir,
        ] {
            fs::create_dir_all(dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(dir, perms)?;
            }
        }
        Ok(())
    }

    /// Product/diagnostic projection — never includes key material.
    pub fn layout(&self) -> StoreLayout {
        StoreLayout {
            account_segment: self.account_segment.clone(),
            account_root: self.account_root.to_string_lossy().into_owned(),
            state_dir: self.state_dir.to_string_lossy().into_owned(),
            crypto_dir: self.crypto_dir.to_string_lossy().into_owned(),
            cache_dir: self.cache_dir.to_string_lossy().into_owned(),
            media_dir: self.media_dir.to_string_lossy().into_owned(),
        }
    }
}

fn ensure_under_root(root: &Path, candidate: &Path) -> Result<(), StorePathError> {
    // Lexical check: candidate must start with root as a path prefix.
    if candidate == root || candidate.starts_with(root) {
        Ok(())
    } else {
        Err(StorePathError::PathEscapesRoot)
    }
}
