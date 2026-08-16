//! Privacy-safe errors for media export intents (P7.5).

use crate::transport::MatrixIpcErrorCategory;

/// Media export queue failure.
///
/// Variants carry fixed diagnostic identifiers only. Export handles, room ids,
/// paths, and media contents are never included in `Debug` or `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportError {
    Invalid { diagnostic_id: &'static str },
    NotFound { diagnostic_id: &'static str },
}

impl ExportError {
    pub fn diagnostic_id(self) -> &'static str {
        match self {
            Self::Invalid { diagnostic_id } | Self::NotFound { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid media export operation ({diagnostic_id})")
            }
            Self::NotFound { diagnostic_id } => {
                write!(f, "media export job not found ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for ExportError {}
