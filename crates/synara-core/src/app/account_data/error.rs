//! Privacy-safe errors for account-data service (P6.7).

use crate::transport::MatrixIpcErrorCategory;

/// Account-data index / apply failure.
#[derive(Debug)]
pub enum AccountDataError {
    Invalid {
        diagnostic_id: &'static str,
    },
    NotFound {
        diagnostic_id: &'static str,
    },
    Forbidden {
        diagnostic_id: &'static str,
    },
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
}

impl AccountDataError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::NotFound { diagnostic_id }
            | Self::Forbidden { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::Invalid { .. } | Self::NotFound { .. } | Self::Forbidden { .. } => {
                MatrixIpcErrorCategory::SdkInvariant
            }
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
        }
    }
}

impl std::fmt::Display for AccountDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid account data operation ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => {
                write!(f, "account data not found ({diagnostic_id})")
            }
            Self::Forbidden { diagnostic_id } => {
                write!(f, "forbidden account data content ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale account data generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
        }
    }
}

impl std::error::Error for AccountDataError {}
