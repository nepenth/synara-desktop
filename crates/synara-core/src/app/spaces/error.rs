//! Privacy-safe errors for space hierarchy projection (P4.5).

use crate::transport::MatrixIpcErrorCategory;

/// Space hierarchy foundation failure.
#[derive(Debug)]
pub enum SpaceError {
    Invalid { diagnostic_id: &'static str },
    NotFound { diagnostic_id: &'static str },
    Cycle { diagnostic_id: &'static str },
}

impl SpaceError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id }
            | Self::NotFound { diagnostic_id }
            | Self::Cycle { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for SpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "space hierarchy error ({})", self.diagnostic_id())
    }
}

impl std::error::Error for SpaceError {}
