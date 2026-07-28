//! Privacy-safe errors for timeline filter (P5.11).

use crate::matrix::ipc::MatrixIpcErrorCategory;

/// Timeline filter failure.
#[derive(Debug)]
pub enum TimelineFilterError {
    Invalid { diagnostic_id: &'static str },
}

impl TimelineFilterError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for TimelineFilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid timeline filter ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for TimelineFilterError {}
