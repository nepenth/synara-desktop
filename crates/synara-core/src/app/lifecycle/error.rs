//! Privacy-safe errors for destructive lifecycle operations (P2.6).
//!
//! Errors never carry tokens, store encryption keys, recovery keys, or raw
//! secret material. Paths appear only as layout segments / diagnostic codes.

use std::io;

use crate::app::store::StoreKeyVaultError;
use crate::transport::MatrixIpcErrorCategory;

/// Destructive lifecycle failure (logout / wipe / recovery).
#[derive(Debug)]
pub enum LifecycleError {
    /// Identity or path resolution failed before any destructive I/O.
    InvalidTarget { diagnostic_id: &'static str },
    /// Proposed wipe path is not the exact derived account root.
    TargetMismatch { diagnostic_id: &'static str },
    /// Path escapes the configured Matrix store root (traversal defense).
    PathEscapesRoot { diagnostic_id: &'static str },
    /// Wipe refused for a policy reason (root wipe, symlink, sibling risk).
    WipeRefused {
        diagnostic_id: &'static str,
        reason: &'static str,
    },
    /// Filesystem I/O failure (no secret content).
    Io(io::Error),
    /// Store-key vault backend failure (no key bytes).
    StoreKeyVault(StoreKeyVaultError),
    /// Session material vault backend failure.
    Vault {
        diagnostic_id: &'static str,
        category: MatrixIpcErrorCategory,
    },
    /// Supervisor transition failed.
    Supervisor {
        diagnostic_id: &'static str,
        detail: String,
    },
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTarget { diagnostic_id } => {
                write!(f, "invalid lifecycle target ({diagnostic_id})")
            }
            Self::TargetMismatch { diagnostic_id } => {
                write!(f, "exact wipe target mismatch ({diagnostic_id})")
            }
            Self::PathEscapesRoot { diagnostic_id } => {
                write!(f, "lifecycle path escapes root ({diagnostic_id})")
            }
            Self::WipeRefused {
                diagnostic_id,
                reason,
            } => write!(f, "local wipe refused ({diagnostic_id}): {reason}"),
            Self::Io(e) => write!(f, "lifecycle io error: {e}"),
            Self::StoreKeyVault(e) => write!(f, "store key vault error: {e}"),
            Self::Vault {
                diagnostic_id,
                category,
            } => write!(f, "session vault error ({category:?}, {diagnostic_id})"),
            Self::Supervisor {
                diagnostic_id,
                detail,
            } => write!(f, "supervisor error ({diagnostic_id}): {detail}"),
        }
    }
}

impl std::error::Error for LifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::StoreKeyVault(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for LifecycleError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<StoreKeyVaultError> for LifecycleError {
    fn from(value: StoreKeyVaultError) -> Self {
        Self::StoreKeyVault(value)
    }
}
