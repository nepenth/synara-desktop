//! Privacy-safe errors for room directory session (P6.10).

use crate::transport::MatrixIpcErrorCategory;

/// Room directory session / apply failure.
#[derive(Debug)]
pub enum RoomDirectoryError {
    Invalid {
        diagnostic_id: &'static str,
    },
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
    Cancelled {
        diagnostic_id: &'static str,
    },
}

impl RoomDirectoryError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. }
            | Self::Cancelled { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } => MatrixIpcErrorCategory::SdkInvariant,
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
            Self::Cancelled { .. } => MatrixIpcErrorCategory::Cancellation,
        }
    }
}

impl std::fmt::Display for RoomDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid room directory operation ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale room directory generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
            Self::Cancelled { diagnostic_id } => {
                write!(f, "room directory cancelled ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for RoomDirectoryError {}
