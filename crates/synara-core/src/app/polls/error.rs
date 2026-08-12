//! Privacy-safe errors for poll and state projection (P5.7).

use crate::transport::MatrixIpcErrorCategory;

/// Poll/state projection validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    Invalid { diagnostic_id: &'static str },
}

impl ProjectionError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid poll/state projection ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for ProjectionError {}
