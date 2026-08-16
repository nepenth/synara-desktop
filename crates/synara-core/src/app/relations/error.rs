//! Privacy-safe errors for relation projection (P5.6).

use crate::transport::MatrixIpcErrorCategory;

/// Relation index / apply failure.
#[derive(Debug)]
pub enum RelationError {
    Invalid {
        diagnostic_id: &'static str,
    },
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
}

impl RelationError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id } | Self::StaleGeneration { diagnostic_id, .. } => {
                diagnostic_id
            }
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } => MatrixIpcErrorCategory::SdkInvariant,
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
        }
    }
}

impl std::fmt::Display for RelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid relation operation ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale relation generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
        }
    }
}

impl std::error::Error for RelationError {}
