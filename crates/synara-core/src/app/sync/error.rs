//! Privacy-safe errors for the sync readiness foundation (P4.1).
//!
//! Errors never carry tokens, homeserver raw bodies, or store paths.

use crate::transport::MatrixIpcErrorCategory;

/// Sync readiness / reconnect foundation failure.
#[derive(Debug)]
pub enum SyncError {
    /// Client is not authenticated; SyncService requires a live session.
    NotAuthenticated { diagnostic_id: &'static str },
    /// SyncService already installed / ownership conflict for this generation.
    AlreadyRunning { diagnostic_id: &'static str },
    /// No SyncService is currently owned (stop/restart without start).
    NotRunning { diagnostic_id: &'static str },
    /// Generation stamp mismatch (stale session epoch).
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
    /// SDK SyncService build/start/stop failure (privacy-safe code only).
    Sdk {
        diagnostic_id: &'static str,
        category: MatrixIpcErrorCategory,
    },
    /// Invalid operator input / policy violation.
    Invalid { diagnostic_id: &'static str },
}

impl SyncError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::NotAuthenticated { diagnostic_id }
            | Self::AlreadyRunning { diagnostic_id }
            | Self::NotRunning { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. }
            | Self::Sdk { diagnostic_id, .. }
            | Self::Invalid { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::NotAuthenticated { .. } => MatrixIpcErrorCategory::AuthenticationRejected,
            Self::AlreadyRunning { .. } | Self::NotRunning { .. } | Self::Invalid { .. } => {
                MatrixIpcErrorCategory::SdkInvariant
            }
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
            Self::Sdk { category, .. } => *category,
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAuthenticated { diagnostic_id } => {
                write!(f, "sync requires authenticated client ({diagnostic_id})")
            }
            Self::AlreadyRunning { diagnostic_id } => {
                write!(f, "sync service already running ({diagnostic_id})")
            }
            Self::NotRunning { diagnostic_id } => {
                write!(f, "sync service not running ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale sync generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
            Self::Sdk {
                diagnostic_id,
                category,
            } => write!(f, "sync sdk error ({category:?}, {diagnostic_id})"),
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid sync operation ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for SyncError {}
