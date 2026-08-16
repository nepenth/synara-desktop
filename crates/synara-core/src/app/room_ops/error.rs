//! Privacy-safe errors for room membership ops queue (P6.9).

use crate::transport::MatrixIpcErrorCategory;

/// Room ops queue failure.
#[derive(Debug)]
pub enum RoomOpsError {
    Invalid {
        diagnostic_id: &'static str,
    },
    NotFound {
        diagnostic_id: &'static str,
    },
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
}

impl RoomOpsError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::NotFound { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } | Self::NotFound { .. } => MatrixIpcErrorCategory::SdkInvariant,
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
        }
    }
}

impl std::fmt::Display for RoomOpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid room op ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => {
                write!(f, "room op not found ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale room op generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
        }
    }
}

impl std::error::Error for RoomOpsError {}
