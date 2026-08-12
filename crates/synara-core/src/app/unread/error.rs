//! Privacy-safe errors for unread / read-marker positioning (P5.5).

use crate::transport::MatrixIpcErrorCategory;

/// Unread positioning failure.
#[derive(Debug)]
pub enum UnreadError {
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

impl UnreadError {
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

impl std::fmt::Display for UnreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid unread operation ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => {
                write!(f, "unread state not found ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale unread generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
        }
    }
}

impl std::error::Error for UnreadError {}
