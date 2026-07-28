//! Privacy-safe errors for MatrixRTC call-state projection (P10.4).

use crate::matrix::ipc::MatrixIpcErrorCategory;

/// Call-state projection validation or lookup failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallStateError {
    Invalid { diagnostic_id: &'static str },
    NotFound { diagnostic_id: &'static str },
}

impl CallStateError {
    pub fn diagnostic_id(self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id } | Self::NotFound { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for CallStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid call-state projection ({})",
            self.diagnostic_id()
        )
    }
}

impl std::error::Error for CallStateError {}
