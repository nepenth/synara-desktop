//! Privacy-safe errors for media upload / download queues (P6.4 / P7.2).

use crate::transport::MatrixIpcErrorCategory;

/// Media upload or download queue failure.
#[derive(Debug)]
pub enum MediaError {
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

impl MediaError {
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

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid media operation ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => {
                write!(f, "media job not found ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale media generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
        }
    }
}

impl std::error::Error for MediaError {}
