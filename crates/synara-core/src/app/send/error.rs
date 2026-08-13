//! Privacy-safe errors for the send-queue foundation (P6.1).

use crate::transport::MatrixIpcErrorCategory;

/// Outbound message queue failure.
#[derive(Debug)]
pub enum SendError {
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
    Failed {
        diagnostic_id: &'static str,
        category: MatrixIpcErrorCategory,
    },
}

impl SendError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::NotFound { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. }
            | Self::Failed { diagnostic_id, .. } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } | Self::NotFound { .. } => MatrixIpcErrorCategory::SdkInvariant,
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
            Self::Failed { category, .. } => *category,
        }
    }
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid send operation ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => write!(f, "send item not found ({diagnostic_id})"),
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale send generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
            Self::Failed {
                diagnostic_id,
                category,
            } => write!(f, "send failed ({category:?}, {diagnostic_id})"),
        }
    }
}

impl std::error::Error for SendError {}
