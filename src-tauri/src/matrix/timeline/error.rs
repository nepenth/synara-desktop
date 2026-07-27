//! Privacy-safe errors for timeline registry (P5.1).

use crate::matrix::ipc::MatrixIpcErrorCategory;

/// Timeline registry / lifecycle failure.
#[derive(Debug)]
pub enum TimelineError {
    Invalid {
        diagnostic_id: &'static str,
    },
    NotFound {
        diagnostic_id: &'static str,
    },
    AlreadyOpen {
        diagnostic_id: &'static str,
    },
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
    Failed {
        diagnostic_id: &'static str,
        category: MatrixIpcErrorCategory,
    },
}

impl TimelineError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::NotFound { diagnostic_id }
            | Self::AlreadyOpen { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. }
            | Self::Failed { diagnostic_id, .. } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } | Self::NotFound { .. } | Self::AlreadyOpen { .. } => {
                MatrixIpcErrorCategory::SdkInvariant
            }
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
            Self::Failed { category, .. } => *category,
        }
    }
}

impl std::fmt::Display for TimelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid timeline operation ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => write!(f, "timeline not found ({diagnostic_id})"),
            Self::AlreadyOpen { diagnostic_id } => {
                write!(f, "timeline already open ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale timeline generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
            Self::Failed {
                diagnostic_id,
                category,
            } => write!(f, "timeline failed ({category:?}, {diagnostic_id})"),
        }
    }
}

impl std::error::Error for TimelineError {}
