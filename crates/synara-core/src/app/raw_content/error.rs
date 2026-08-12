//! Privacy-safe errors for custom raw-content extraction (P5.9).

use crate::transport::MatrixIpcErrorCategory;

/// Raw-content extraction failure.
#[derive(Debug)]
pub enum RawContentError {
    Invalid {
        diagnostic_id: &'static str,
    },
    ForbiddenField {
        diagnostic_id: &'static str,
    },
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
}

impl RawContentError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::ForbiddenField { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } | Self::ForbiddenField { .. } => {
                MatrixIpcErrorCategory::SdkInvariant
            }
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
        }
    }
}

impl std::fmt::Display for RawContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid raw content operation ({diagnostic_id})")
            }
            Self::ForbiddenField { diagnostic_id } => {
                write!(f, "forbidden raw content field ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale raw content generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
        }
    }
}

impl std::error::Error for RawContentError {}
