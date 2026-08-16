//! Privacy-safe errors for route / deep-link resolution (P4.8).

use crate::transport::MatrixIpcErrorCategory;

/// Route parse / build failure.
#[derive(Debug)]
pub enum RouteError {
    Invalid { diagnostic_id: &'static str },
}

impl RouteError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid route operation ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for RouteError {}
