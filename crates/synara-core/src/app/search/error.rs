//! Privacy-safe errors for search projection (P6.8).

use crate::transport::MatrixIpcErrorCategory;

/// Search session / apply failure.
#[derive(Debug)]
pub enum SearchError {
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

impl SearchError {
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

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid search operation ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale search generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
            Self::Cancelled { diagnostic_id } => {
                write!(f, "search cancelled ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for SearchError {}
